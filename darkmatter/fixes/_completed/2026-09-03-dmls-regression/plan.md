---
total_phases: 6
created: 2026-09-03
phase: 1
agent: codex/default
yolo: true
packages:
  - dmls
  - zed-dmls-cli
---

# Execution Plan: Make the Zed DMLS Launch Path Testable

This plan implements the reviewed design in `spec.md`. It does not change DMLS
protocol behavior or the VS Code extension. It adds four independent proofs:
the checked-in Zed extension has a loadable shape, it compiles for Zed's real
WASI target, Zed's own pinned packager accepts it, and a host-side CLI can stage
and diagnose the dev-extension registration without tying Zed to a disposable
worktree.

## Phase 1: Lock Contracts, Scope, and Package Structure

- [ ] **Task 1.1: Capture the pre-change baseline and required blast-radius evidence.** Record `git status --short`, the current DMLS and Zed targeted test results, and `sniff repo package-areas`. Before editing existing Rust or Python symbols, run GitNexus upstream impact analysis for each symbol to be changed; report any HIGH or CRITICAL result before proceeding. Save a pre-change `detect_changes` result so later scope review can distinguish this work from the already-untracked fix directory.

- [ ] **Task 1.2: Define the `zed-dmls-cli` package boundary.** Add `darkmatter/dmls/zed-dmls-cli` as a normal workspace package with a library surface for testable staging/diagnostic logic and a thin `zed-dmls` binary for Clap parsing, rendering, and exit-code translation. Keep `darkmatter/dmls/zed-dmls` workspace-excluded. Add only the dependencies required by the specification: focused `sniff` OS/program discovery, `dirs` user-data paths, `biscuit-terminal` rendering, Clap, TOML parsing, and test-only temporary filesystem support.

- [ ] **Task 1.3: Register the new package everywhere package inventory is authoritative.** Update root workspace membership, Darkmatter area build/test/lint/sanity package lists, root and area `docs/dependencies.md`, and the Darkmatter package overview. Give `zed-dmls-cli` ordinary L1 CI coverage on macOS, Linux, and Windows; keep the existing `dmls` L2 tiers, tmux backend, and Neovim runner tool unchanged.

- [ ] **Task 1.4: Establish test seams and exit semantics before behavior.** Define injectable traits or focused adapters for OS discovery, executable lookup/version execution, filesystem/link resolution, path defaults, and bounded log reads. Define stable result states and reserve exit code `0` for healthy, `1` for staging/diagnostic failure, `2` for Clap usage, and `3` for successful staging that still requires manual Zed registration. Ensure `--plain`, `--staging-dir`, `--zed-data-dir`, and `--zed-log` are accepted without consulting ambient host state in argument parsing.

- [ ] **Validation checkpoint 1:** Run Cargo metadata and the CI scope-policy tests. Confirm `zed-dmls-cli` is a workspace member on all runner OSes, `zed-dmls` remains excluded, the existing DMLS L2 policy is byte-for-byte equivalent except for later addition of the packager runner tool, and the CLI skeleton's help/usage tests pass without launching Zed or opening a terminal.

## Phase 2: Add Passive Extension Contracts and Correct the WASI Check

- [ ] **Task 2.1: Add the cross-platform Zed manifest contract test.** Create `darkmatter/dmls/tests/zed_extension_contract.rs` in the style of `packaging_contract.rs`. Parse `zed-dmls/extension.toml` and `zed-dmls/Cargo.toml` with `toml`; assert `id = "dmls"`, schema version 1, non-empty name/version, manifest/package version equality, the exact Markdown language-server mapping, `crate-type = ["cdylib"]`, a `zed_extension_api` dependency, one matching package in `Cargo.lock`, and an existing `src/lib.rs`. Do not inspect implementation source strings or impose negative rules on `vscode-dmls`. **Parallelizable:** after Phase 1, this can proceed alongside Tasks 2.2-2.3 because it touches only the passive DMLS test surface.

- [ ] **Task 2.2: Make `check-zed` authoritative and non-mutating.** Change the Darkmatter recipe to `wasm32-wasip2` and `cargo check --locked --manifest-path ./dmls/zed-dmls/Cargo.toml --target wasm32-wasip2`. If the target is missing, fail with the exact `rustup target add wasm32-wasip2` remedy; never install it or skip. Add `check-zed` after native package lint calls in the area `lint` recipe. **Parallelizable:** this and Task 2.3 can be implemented together independently of Task 2.1.

- [ ] **Task 2.3: Provision the correct target where the area gate is scheduled.** Add `wasm32-wasip2` to the existing Rust setup action in `rust-latest-stable.yml` before the Darkmatter area lint gate. Keep toolchain provisioning separate from verification so local `just lint` remains read-only with respect to rustup.

- [ ] **Validation checkpoint 2:** Run the new DMLS test through nextest. Prove non-vacuity by separately deleting `extension.toml`, changing its `id`, and removing `[language_servers.dmls]`, observing a red test for each mutation, and restoring each file immediately. With the WASI target installed, run `just check-zed`; also test the missing-target branch with an injected/isolated rustup view and confirm it fails with the exact install command without mutating the toolchain. Verify `git grep wasip1 darkmatter/` is empty outside historical fix/feature records that are intentionally immutable.

## Phase 3: Add the Official Zed Packaging Companion Gate

- [ ] **Task 3.1: Pin one reviewed Zed packager supply-chain record.** Add a single versioned configuration under `.github/ci/` containing the Zed commit SHA, official Linux x86_64 `zed-extension` download URL, and expected SHA-256. Use the same record from reusable package CI and `rust-latest-stable.yml`; reject missing fields, download failure, or digest mismatch. Cache by the Zed SHA and do not compile a floating Zed checkout as fallback.

- [ ] **Task 3.2: Add local packaging and verification recipes.** Add `zed-package` to invoke an already-provisioned `zed-extension` binary against `dmls/zed-dmls`, and `zed-verify` to run `check-zed`, package into sibling temporary output/scratch directories, parse `manifest.json`, and inspect `archive.tar.gz`. Require the packaged manifest to identify `dmls` and Markdown and the archive to contain `extension.wasm`; clean temporary directories on success, command failure, and validation failure. Do not classify either recipe as L2. **Parallelizable:** after Task 3.1 fixes the packager contract, this can proceed alongside Task 3.3.

- [ ] **Task 3.3: Extend the CI policy vocabulary and ownership tests.** Add `zed-extension` to `KNOWN_RUNNER_TOOLS`, document it in `.github/ci/README.md` as a package companion verification tool rather than an L2 backend, and add it to `dmls` metadata beside `neovim`. Extend `scripts/ci/test_affected_scope.py` to prove the tool is accepted, a path below workspace-excluded `darkmatter/dmls/zed-dmls/**` selects `dmls`, and the resulting DMLS record carries the Zed verification requirement without propagating it to dependents. **Parallelizable:** after Phase 2, this policy/test work can proceed alongside Task 3.2.

- [ ] **Task 3.4: Wire mandatory Ubuntu packaging into both workflows.** In `_package-ci.yml`'s DMLS lint producer, conditionally provision `wasm32-wasip2`, restore/download the pinned packager, verify its SHA-256 before placing it on `PATH`, and run `just zed-verify` in a named step whenever `runner-tools` contains `zed-extension`. Ensure a failed or skipped required Zed step makes the lint producer red and leaves attributable status evidence. Provision from the same pin and run the same recipe in the weekly latest-stable workflow.

- [ ] **Validation checkpoint 3:** Run CI-policy/unit tests and a local Linux-compatible dry run of packager provisioning and `zed-verify`. Demonstrate that an unavailable tool, altered expected digest, wrong packaged id/language, or missing `extension.wasm` fails rather than skips. Confirm no `BISCUIT_TEST_LEVEL_REQUIRED`, `require_level!`, real terminal, display, or Zed process participates in this gate.

## Phase 4: Implement Cross-Platform Stable Staging

- [ ] **Task 4.1: Implement one-request host/path discovery.** Use the focused `sniff` API once per command to identify the platform and locate executables; use `dirs` to derive the default per-user data root. Resolve the stable `dmls/zed-dmls` staging path under macOS Application Support, Linux XDG data, or Windows local app data using only `Path`/`PathBuf`. Make explicit CLI overrides bypass defaults so tests and Zed Preview/custom installations remain hermetic.

- [ ] **Task 4.2: Implement an allowlisted staging plan.** Copy only `extension.toml`, `Cargo.toml`, `Cargo.lock`, `src/`, and `README.md` from the checked-in extension. Explicitly exclude the checked-in/generated `extension.wasm`, `target/`, and all unrecognized files. Validate the source manifest before modifying the destination and report the exact stable folder the user must select.

- [ ] **Task 4.3: Make directory replacement rollback-safe across OSes.** Build a complete sibling temporary directory, move any existing destination to a sibling backup, rename the new directory into place, and restore the backup if the final swap fails. Remove temporary/backup directories only after the outcome is known. Do not rely on Unix-only atomic replacement, separators, symlink APIs, shell copying, or ambient CWD.

- [ ] **Task 4.4: Cover staging as pure L1 behavior.** Add injected-filesystem tests for fresh staging, repeat staging after `src/lib.rs` changes, removal of stale destination files, allowlist enforcement, manifest rejection before mutation, swap failure with rollback, and cleanup after success/failure. Parameterize macOS, Linux/XDG, and Windows path shapes. Assert staging never writes under Zed's `extensions/installed`, launches Zed, changes the clipboard, or requests host input.

- [ ] **Validation checkpoint 4:** Run the `zed-dmls-cli` unit/integration tests with nextest on the host and cross-check compilation for the repository's macOS, Linux, and Windows CI targets where available. In a temporary tree, run `stage` twice around a source edit and verify the stable copy updates, stale files disappear, rollback retains the prior usable copy under injected failure, and exit code `3` is returned only when manual registration remains.

## Phase 5: Implement Doctor Diagnostics and Installation Integration

- [ ] **Task 5.1: Implement structured binary health checks.** Locate `dmls` through focused discovery, run bounded `dmls --version`, and compare it with the package-compatible version. Report missing and incompatible binaries as failures. A same-version binary may be called `version-compatible`; call it `current` only when a trustworthy installation receipt proves provenance/freshness, and otherwise state that freshness is unverified. **Parallelizable:** using the Phase 1 seams, Tasks 5.1-5.3 can be implemented independently before Task 5.4 assembles their report.

- [ ] **Task 5.2: Implement registration and manifest health checks.** Derive or accept the Zed data directory, require `extensions/installed/dmls`, and resolve its target through filesystem abstractions that cover symlinks, Windows junctions, and other supported links. Preserve and report the registered target when resolution fails. Parse the resolved `extension.toml` and require `id = "dmls"`; distinguish absent registration, dangling worktree target, missing manifest, and wrong-id/wrong-folder states with the remedies specified in the functional specification.

- [ ] **Task 5.3: Add bounded, corroborating log diagnostics.** Read only the newest bounded Zed log tail from the default/overridden log. Recognize the two observed manifest-error forms, quote the implicated extension id, and distinguish `dmls` startup breakage from a different folder id selected during install. Treat old errors as historical context when current registration is valid; make log evidence fatal only when it corroborates a missing or invalid current registration.

- [ ] **Task 5.4: Render all user-facing output through `TerminalRenderable`.** Build the default report from `Prose`, `UnorderedList`, or `Table` components and provide deterministic `--plain` output for automation/snapshots. Keep stdout/stderr separation and exit codes stable; do not emit handwritten ANSI sequences. Add output tests for every failure message and the healthy-with-historical-error case.

- [ ] **Task 5.5: Integrate thin just recipes without coupling editor-neutral install success to Zed.** Add `install-zed` as `zed-dmls stage` followed by `zed-dmls doctor`, and `zed-doctor` as the doctor wrapper. Extend `install-dmls` to invoke doctor only when Zed or an existing DMLS dev registration is detected; surface doctor failure as a warning while preserving the successful binary-install exit status. `install-zed` must fail on staging failure and return the documented manual-registration status after successful staging when registration is absent; it must never edit `extensions/installed`.

- [ ] **Validation checkpoint 5:** Run injected tests for missing Zed, missing/incompatible/same-version-unreceipted binary, absent registration, dangling link/junction-equivalent, missing manifest, wrong id, wrong-folder log entry, historical resolved error, and fully healthy state. Exercise `install-dmls`, `install-zed`, and `zed-doctor` against temporary overrides and assert exact messages, paths, and exit statuses. No test may launch/focus Zed, open/focus a terminal, mutate the clipboard, or accept interactive input.

## Phase 6: Synchronize Documentation and Close the Acceptance Matrix

- [ ] **Task 6.1: Replace worktree-bound install guidance.** Update `darkmatter/dmls/zed-dmls/README.md`, `darkmatter/dmls/docs/editors/zed.md`, and the DMLS README editor table to prescribe `just install-zed`, selecting the printed stable folder, and `just zed-doctor`. Explain the `dmls` startup error as a dangling registration and an error naming another directory (notably `vscode-dmls`) as the wrong folder selected. Document `wasm32-wasip2`, manual-registration exit code `3`, Preview/custom path overrides, and the fact that the command never modifies Zed registration. **Parallelizable:** draft alongside final Phase 5 tests, then reconcile wording against the verified output and exit behavior before checkpoint 6.

- [ ] **Task 6.2: Synchronize developer and dependency guidance.** Update `.claude/skills/darkmatter/dmls.md` with the L1 contract, mandatory `check-zed`, and official packager gate. Finish root/area dependency and package-list documentation for `zed-dmls-cli`, including `sniff`, `dirs`, and `biscuit-terminal`, and remove current statements that `zed-dmls` is not checked by monorepo recipes.

- [ ] **Task 6.3: Run package-scoped quality gates.** From `darkmatter/`, run `just build`, `just test`, and `just lint`; run `just zed-verify` in the provisioned Linux/CI-equivalent environment. Run the CI scope test suite and validate the latest-stable workflow syntax/config consumption. Do not run `cargo fmt`; inspect touched files for formatting and warnings without rewriting unrelated code.

- [ ] **Task 6.4: Execute the full regression matrix.** Re-run all three manifest non-vacuity mutations, missing-WASI-target behavior, packager digest/artifact negative cases, every staging rollback/repeat case, and every doctor fixture. On a disposable real-host setup when available, stage from worktree A, register the stable directory manually, remove worktree A, restart Zed, and confirm no new `No extension manifest found for extension dmls` line; record this as manual evidence, not an automated CI requirement.

- [ ] **Task 6.5: Audit final scope and comments.** Run GitNexus `detect_changes` against `main`, inspect `git diff`, and confirm changes are limited to DMLS/Zed contracts, `zed-dmls-cli`, CI provisioning/scope, recipes, and named documentation. Revisit all touched docs/comments for behavior drift, confirm no VS Code extension behavior changed, and preserve unrelated worktree edits. Do not commit unless separately requested.

- [ ] **Validation checkpoint 6:** Map all nine acceptance criteria in `spec.md` to named passing tests, workflow steps, commands, or the one explicit manual host check. Completion requires cross-platform L1 manifest/CLI evidence, mandatory Ubuntu official packaging evidence, zero unintended `wasm32-wasip1` guidance, a stable worktree-independent staged path, and actionable doctor output for both original incident signatures.
