---
kind: plan
total_phases: 4
created: 2026-11-08
phase: 5
agent: codex/gpt-6.1-sol
yolo: 'true'
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
- .github/workflows/_package-ci.yml
- biscuit-visualized/src/src/tests/embed_render_tests.rs
- claudine/cli/tests/l1/compose_schema_cli.rs
- darkmatter/cli/tests/l1/invalid_trigger_warnings.rs
- darkmatter/lib/tests/l1/kind_semantic_checks.rs
- messenger/lib/tests/research_refresh.rs
- tools/test-toolkit/tests/ci_workflow_contracts.rs
- tree-hugger/cli/src/changed.rs
- tree-hugger/cli/tests/changed/index_corruption.rs
- tree-hugger/cli/tests/changed/ownership.rs
- unchained-ai/model_id/Cargo.toml
- unchained-ai/model_id/tests/ui.rs
- worktree/cli/tests/list_output.rs
- worktree/lib/src/git/program.rs
- worktree/lib/src/remove/lifecycle/tests.rs
docs_updated_during_phase_2:
- tree-hugger/docs/symbol-reports/changed-files.md
- worktree/docs/performance-testing.md
docs_created_during_phase_2:
- unchained-ai/docs/testing.md
- worktree/docs/git-for-windows.md
skills_files_updated_during_phase_2:
- .claude/skills/os/SKILL.md
- .claude/skills/os/windows-paths.md
- .claude/skills/os/windows.md
- .claude/skills/worktree/SKILL.md
packages:
- biscuit-visualized
- claudine-cli
- darkmatter
- darkmatter-cli
- git-wt-cli
- messenger
- model_id
- test-toolkit
- tree-hugger-cli
- worktree
source_files_during_phase_3: []
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
- .claude/skills/os/windows-paths.md
- .claude/skills/os/wsl.md
source_files_during_phase_4: []
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
- .claude/skills/os/windows.md
source_code:
- .github/workflows/_package-ci.yml
- biscuit-visualized/src/src/tests/embed_render_tests.rs
- claudine/cli/tests/l1/compose_schema_cli.rs
- darkmatter/cli/tests/l1/invalid_trigger_warnings.rs
- darkmatter/lib/tests/l1/kind_semantic_checks.rs
- messenger/lib/tests/research_refresh.rs
- tools/test-toolkit/tests/ci_workflow_contracts.rs
- tree-hugger/cli/src/changed.rs
- tree-hugger/cli/tests/changed/index_corruption.rs
- tree-hugger/cli/tests/changed/ownership.rs
- unchained-ai/model_id/Cargo.toml
- unchained-ai/model_id/tests/ui.rs
- worktree/cli/tests/list_output.rs
- worktree/lib/src/git/program.rs
- worktree/lib/src/remove/lifecycle/tests.rs
documentation:
- tree-hugger/docs/symbol-reports/changed-files.md
- worktree/docs/performance-testing.md
- unchained-ai/docs/testing.md
- worktree/docs/git-for-windows.md
- .claude/skills/os/SKILL.md
- .claude/skills/os/windows-paths.md
- .claude/skills/os/windows.md
- .claude/skills/os/wsl.md
- .claude/skills/worktree/SKILL.md
completed_phase: 4
implemented: true
---

# Windows and WSL2 failure repair plan

## Summary and Definition of Done

Repair the Windows failures without changing path presentation contracts, support both launcher and direct Git for Windows discovery, make compiler-test prerequisites explicit, and remove the messenger fixture's shell-startup race. Execute restored WSL2 archives and repair archive-relocation failures within this scope. Preserve required coverage and report unavailable capabilities separately from executed tests.

Completion requires a disposition for the compile failure and every one of the original 141 failed tests; focused passing reproductions on the affected OS; passing affected-area L1 and lint checks; relocation evidence from Ubuntu-built archives executed on toolchain-free WSL2 with builder paths unavailable; and three consecutive qualifying green scheduled runs on main for each of native Windows and WSL2. Each confirmation must identify executed tests and reused cells. Empty plans, early-return tests, and accepted gaps are not passing behavior evidence. An unmet capability or unavailable scheduled confirmation leaves the corresponding acceptance item incomplete, even when code is ready for review.

The implementation runs all phases without human gates. It produces an implementation log and a final review record; it does not commit, push, create branches, move lifecycle directories, or trigger full-workspace workflows. Existing scheduled runs supply main confirmation. Preserve unrelated working-tree edits, particularly the in-progress archive workflow and planner changes present at planning time.

## Phase 1: Establish evidence and contracts

### Necessary Rules

1. **Reuse completed diagnosis.** `spikes/S5-windows-repro.md` records nightly run `42246114963`, commit `a816f1447`, exact failure groups, and native Windows long/short TEMP reproductions. `spikes/S5b-ucrt64.md` disproves the single-layout diagnosis. Import those results into `implementation-log.md`; do not repeat their timing study or schedule another discovery spike. The spec's provisional table remains a historical snapshot.
2. **Path fixes belong to tests.** In `tree-hugger/cli/tests/changed/index_corruption.rs::assert_reports_reject`, expect the full simplified canonical repository path in the diagnostic. Resolve it with `biscuit_file::canonicalize_simplified(...).expect(...)`, preserving the no-partial-report and pre-cache failure assertions. The S5 short-TEMP reproduction proves the product already reports the canonical long spelling.
3. **Preserve relative symlink traversal.** In `tree-hugger/cli/tests/changed/ownership.rs::parent_traversal`, replace slash-containing relative target literals with component-by-component `join` calls, including directory links, recursive aliases, dot-parent, and ordinary parent traversal. Retain `.` and `..` in the target supplied to the OS; do not canonicalize it first. Keep outside-owner rejection, ordinary-directory acceptance, and identity controls. The current `link` comment already requires this, but `parent_traversal` violates it. Native Windows permits creating these broken slash-target links; S5 proves they cannot be followed.
4. **Lifecycle diagnostics name the resolved location.** In `worktree/lib/src/remove/lifecycle/tests.rs::assert_changed`, compute the expected full portable path from the existing resolved parent plus the entry name. This works for deleted/replaced entries without resolving the entry itself. Inspect `Refusal::Replaced` structurally and keep exit class and sentence assertions. Do not normalize the entire diagnostic with backslash replacement. All helper callers, including the admin-entry and link-replacement cases, retain their original refusal guarantees.
5. **Launch spelling remains observable.** In `claudine/cli/tests/l1/compose_schema_cli.rs`'s shipped-plan-route assertion around `spec_dir`, derive the expected absolute route from the child's staged launch context and fixture spelling, then use `biscuit_file::to_portable_string`. Remove canonicalization that expands short TEMP aliases. Keep the complete target instruction and negative retargeting assertion. In `darkmatter/cli/tests/l1/invalid_trigger_warnings.rs`, make `write` return a native component-built path retaining the fixture root's spelling; assert that full native trigger spelling in both the child-only warning and shared warning checks. Delete the raw-or-canonical alternative. S5 records the actual child outputs for both suites.
6. **Definition labels are native.** `darkmatter/lib/src/markdown/schemas/yaml_documents/registry.rs::DefinitionSource::label` uses `path.display().to_string()` for file definitions. In `kind_semantic_checks.rs::OpenedCopy`, retain the exact copied definition path as a field and compare `control.definition` against its full native spelling. Replace the three string suffix alternatives; keep absolute-path, positive control, and added-property assertions. No public serialization change is required.
7. **Support both Git entry shapes.** Extend the existing resolver in `worktree/lib/src/git/program.rs`, retaining the private name `launcher_layout`, to recognize `<top>/{cmd,bin}/git.exe` and `<top>/<known-prefix>/bin/git.exe`. Add `("ucrt64", "UCRT64")` to `PREFIXES`. For direct input, derive top three parents above the executable and require exactly one installed recognized prefix matching the input. For both shapes, retain `etc/git-bash.config` rejection, ambiguous-layout rejection, missing-binary rejection, unknown-executable fallback, and application/system-directory precedence. This is a product correction: S5b reproduces all three failures with direct mingw64 PATH entries, so launcher-only test setup would hide the defect.
8. **Preserve Git environment semantics.** Add an entry-kind field to private `Direct`, distinguishing launcher and direct inputs. Launcher input keeps the current MSYSTEM, HOME, PLINK_PROTOCOL, and PATH synthesis. Direct input executes the found binary with inherited environment, removing only the EXEPATH hint this process installed. Both inputs supply the same installation hint to gix. `name_installation_for_gix` and `forget_installation_hint` retain their existing ownership rules and safety contract. A direct binary must not acquire launcher-only environment changes merely because it shares an installation.
9. **Compiler tests must execute or fail explicitly.** Add `[package.metadata.ci] requires-toolchain = true` to `unchained-ai/model_id/Cargo.toml`; keep the `ui` name and both trybuild globs active. Replace the Cargo-unavailable early return in `tests/ui.rs` with an explicit prerequisite assertion. The existing planner's `cargo_toolchain` capability already represents WSL2's governed unavailability; declare the actual requirement without editing its owner, expiry, or availability. WSL2 compiler coverage remains unmet, with a changed execution contract required to close it; do not install Cargo there or describe its accepted gap as a pass.
10. **Fetch before offline tests.** The model_id archive consumer needs registry sources, not merely precompiled dependencies. Add a package-specific prerequisite step in `_package-ci.yml` after pinned-toolchain setup and before execution: `cargo fetch --locked --manifest-path unchained-ai/model_id/Cargo.toml`, guarded by package `model_id` and the resolved cell's `requires_toolchain`. Use the extracted runtime checkout and the same Cargo home as trybuild. This step may use the network; the test process must run with `CARGO_NET_OFFLINE=true`. Do not add a scheduler, cache protocol, or toolchain provisioning to WSL2. Evidence: `ui.rs` performs nested Cargo builds, its manifest requires proc-macro2, and the consumer currently only runs `rustup show` for toolchain prerequisites.
11. **Make font parsing explicit.** In `biscuit-visualized/src/src/tests/embed_render_tests.rs::rule_weights`, use `parse::<u16>()`; the helper returns `Vec<u16>`, so that is the intended numeric type. Record the original E0284 excerpt and confirm the repaired full test target compiles and executes on Windows; a library check does not clear this row.
12. **Remove the messenger startup race at the fixture.** `messenger/lib/tests/research_refresh.rs::a_timed_out_run_settles_its_descendants_before_releasing_the_fleet_lock` starts a PowerShell grandchild inside a two-second elapsed limit; `runner.rs::run` starts its deadline immediately after spawn. Replace this test's shell hierarchy with native Rust helper modes in the same integration-test executable, launched via `current_exe()` and an exact helper-test filter, with process-local environment identifying parent/grandchild roles and paths. The parent spawns the grandchild and remains alive; the grandchild writes PID/readiness then waits for release. Use a bounded readiness channel/file wait, assert the second run is refused while readiness is established, and verify both recorded process identities are gone before releasing the marker and checking no completion file exists. Keep a real elapsed-time stop and existing one-second grace; do not change production deadlines or add timeout allowances, retries, or fixed sleeps. A separate existing test, `the_runner_stops_a_sequence_that_exceeds_the_elapsed_time_limit`, continues to prove deadline enforcement.
13. **No performance spike.** S5b already measured the Git cost and S5 establishes the path failures. Messenger needs one before/after failure reproduction with child status and output, not a benchmark or repeated-until-green run. No broader performance measurement is included.
14. **Archive failures stay within their owner.** Usable Ubuntu-built archives are the prerequisite delivered by `2026-11-06-ubuntu-archive-disk`. Missing archives, storage, and provisioning remain that prerequisite's work. Test failures after execution belong here. Runtime binary references use `biscuit_test_harness::bin_exe!`; repository fixture roots use `manifest_dir!()` and its repository ancestor. Resolver snapshots are anchored at that runtime repository root, not at a nested package or compile-time builder path, following the existing OS skill's WSL archive diagnosis.
15. **No input parser changes are planned.** The input robustness matrix does not apply to path-fixture construction, numeric extraction from generated test CSS, Git filesystem-layout recognition, or provisioning commands. Existing manifest metadata parsing remains unchanged. Do not introduce a new configuration format or loader.

### Wave 1

- [x] **Inventory failures**
  - Create `implementation-log.md` beside this plan. Link the nightly run, commit, failed jobs and individual test identities; copy relevant excerpts from the existing S5 records and retrieve the missing compile/model_id excerpts with read-only GitHub CLI commands (`GIT_TERMINAL_PROMPT=0`). Reconcile the 141 identities plus separate compile failure without counting parameterized controls as failures.
  - Record current source differences from `a816f1447`, existing repairs, and the remaining messenger and WSL2 evidence. Keep a row per identity with cause, repair location, filter, environment, execution status, and evidence link.
- [x] **Record environment**
  - Read declared BUILD variables and OS skill host instructions. At planning time only `BUILD_WSL=build-win` was returned; native Windows and Linux destinations were not declared. Resolve documented host roles from SSH configuration and probe only documented aliases with `BatchMode=yes`; pass matching BUILD variables per invocation, never hardcode them into repository scripts.
  - Record archive-prerequisite status and qualifying existing evidence. Do not mutate standing clones, provision system packages, or run Cargo tests in this phase. Missing access is an explicit unverified acceptance item, not a reason to substitute another OS.

Checkpoint: every original row has an identified repair task and its existing evidence; all contracts above are fixed before edits begin.

## Phase 2: Repair known Windows failures

### Wave 2

These tasks have disjoint implementation ownership and can run concurrently. Delegate bounded tasks to subagents, passing the non-interactive constraint and the applicable rules. Only the coordinator edits the log and shared workflow files. No subagent runs an area-wide suite.

- [x] **Repair tree fixtures**
  - Apply rules 2–3 to index corruption and ownership. Run focused `tree-hugger-cli` filters, proving ordinary traversal still succeeds and foreign ownership still fails. Require Developer Mode or symlink privilege explicitly on Windows; missing privilege is a failure to provision this reproduction.
- [x] **Repair path assertions**
  - Apply rules 4–6 in worktree lifecycle, claudine CLI, darkmatter CLI, and Darkmatter kind checks. Preserve complete message/path contracts and all positive and negative controls. Update drifted nearby comments in the same edit.
- [x] **Repair Git resolution**
  - Apply rules 7–8. Parameterize synthetic tests across all four prefixes, launcher/direct entries, ambiguous installations, git-bash.config, unknown shims, and missing files. Replace the existing direct-binary-is-None assertion with the new supported behavior.
  - Split launcher-environment parity from direct-environment parity: locate the sibling cmd launcher explicitly for the launcher test; compare direct mode with an unwrapped invocation of the found binary for direct mode. Exercise each PATH shape in a fresh subprocess because `windows::resolved` caches once per process. Retain the gix-prefix/child-hint tests and `worktree/cli/tests`' `list_output::a_listing_never_asks_git_for_its_exec_path` guarantee.
  - Perform a real local Git init/status operation on Windows for both entry shapes. Verify exit status, output, gix prefix, and environment capture. Use a current disposable Git for Windows installation containing ucrt64 for ucrt64 parity evidence; do not upgrade the shared host's installation. Update module docs and the area's Git execution topic documentation to cover both entry shapes.
- [x] **Repair compiler prerequisites**
  - Apply rules 9–10. Coordinator owns the narrow workflow insertion, preserving unrelated edits. Add no capability gaps; expose the existing one through truthful package metadata. Document offline nested-build prerequisites in the unchained-ai testing topic page.
  - Reproduce UI execution from an extracted Windows archive with a fresh Cargo home: prerequisite fetch succeeds, then offline compile-pass and compile-fail cases execute. A missing crate must fail before any passing evidence is issued.
- [x] **Repair render compilation**
  - Apply rule 11. Build and execute the containing Windows test binary, including the embedded font/CSS checks; retain its exact compile command and executed test listing.
- [x] **Repair descendant fixture**
  - Capture messenger's original assertion, child status, stdout/stderr, and readiness state once on native Windows. Apply rule 12; helper mode is inert in normal test discovery and never recursively launches itself without an explicit role.
  - Bound every helper lifetime and ensure RAII cleanup on assertion failure. Replace the final fixed sleep with process-exit synchronization followed by the no-write assertion. Record elapsed duration and preserve fleet-lock, stop-reason, process-death, and refused-run assertions.

Checkpoint: targeted tests pass with real execution. Windows path groups run with long TEMP and an 8.3 alias of the same long-named C: directory; S5 establishes that the rig's B: volume cannot supply the alias. Capture both TEMP values and test listings. No whole-message normalization, skipped compiler test, or zero-test filter clears a row.

## Phase 3: Validate relocated WSL2 execution

### Wave 3

- [x] **Execute relocated archives**
  - After the prerequisite supplies usable archives, run `just cross-check <package> --os wsl <test-substring>` for each affected package. Use no Cargo build flags or target selectors, which switch modes or invalidate archive execution. Preserve the toolchain-free guest and unavailable builder target directory; retain producer/consumer manifest identities and archive execution logs.
  - Inventory failures by exact package/test identity and distinguish missing provisioning from test assertions. Apply rule 14 to runtime fixture, binary, and resolver-root failures, updating every caller of the repaired helper rather than adding fallback paths. Keep original expected behavior and assertions. Execute the failed filters again once repaired and record both outcomes.
  - A failure outside those relocation patterns remains a required failed acceptance item with full assertion, child status, and output in the log; do not suppress it, create an unscheduled follow-up, invent a capability gap, or label the overall fix complete. Finish all independent repair and validation tasks regardless.
- [x] **Document platform facts**
  - Update `.claude/skills/os/windows-paths.md` with short-name reproduction and slash-containing relative symlink behavior from S5. Update the Windows topic with launcher/direct Git distinctions and ucrt64 evidence; update the WSL topic with newly verified relocation facts. State current behavior directly without links to dated feature/fix directories.
  - Update package topic docs for public behavior changed by Git discovery. Keep READMEs at overview level. No dependency additions are planned; helper subprocesses use std and existing test infrastructure.

Checkpoint: WSL evidence demonstrates extracted-archive execution, not a normal Linux checkout. model_id's existing toolchain gap is listed as unmet separately; no PASS line from its former early return is reused.

## Phase 4: Verify and assemble review evidence

### Wave 4

- [x] **Run scoped checks**
  - Coordinator runs each changed area's canonical `just test` and `just lint` once after integration: tree-hugger, worktree, claudine, darkmatter, unchained-ai, and biscuit-visualized, plus messenger. Use nextest through the recipes; never run cargo fmt. Shared CI edits get the existing workflow-contract tests through test-toolkit's canonical recipe. Do not add parser-matrix tests for unchanged manifest loaders.
  - Run required affected native Windows and WSL package cells with canonical cross-check tooling. Reuse qualifying receipts by package/environment/tier; filtered reproductions are targeted evidence only. Local macOS passing tests cannot clear Windows or WSL failures. No L2 is introduced by these repairs; existing required L2 checks run only through `just test-l2` without focus changes.
  - Review `just ci-local --plan` and record selected cells and evidence reuse. This is inspection only; this plan authorizes no push or full-workspace diagnostic dispatch.
- [x] **Collect main confirmation**
  - Inspect existing scheduled runs on main and collect three consecutive green qualifying runs per affected OS, recording run URLs, commits, selected packages/tests, reused evidence, and accepted gaps. Only runs containing execution or qualifying reuse of the repaired behavior count. An empty nightly is not one of the three.
  - Record unavailable future confirmations as incomplete acceptance items in the final review record. Do not wait indefinitely, trigger replacement workflows, or claim the outcome complete without those observations.
- [x] **Close evidence ledger**
  - Reconcile every original failure and every restored WSL failure against passing targeted evidence. List remaining capability and access limitations precisely, including the provisioning or execution-contract change needed to remove them.
  - Mark completed task checkboxes, record spec departures and their evidence in the implementation log, and leave the spec snapshot and lifecycle location unchanged. Final status is implementation complete, ready for review only when implementation and scoped verification passed; outcome acceptance remains explicitly incomplete until all required OS confirmation exists.