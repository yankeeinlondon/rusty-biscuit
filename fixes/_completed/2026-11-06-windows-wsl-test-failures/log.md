---
implementation_1: "2026-11-10T17:44:54-08:00"
---

# Review Implementation Log for 2026-11-06-windows-wsl-test-failures

The implementation's phase log is `implementation-log.md` in this directory; this file records the review-to-implement cycles.

## Implementation of Review Findings #1

> **started at:** 2026-11-10T17:44:54-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/rusty-biscuit/fixes/2026-11-06-windows-wsl-test-failures/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review's `implemented` flag was confirmed `false` before repairs started
- the review lists no `human_in_the_loop` items and no blocked findings, so neither finding starts blocked

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1   | ucrt64 Git installation discovery remains broken in gix | fixed | every child row fixed or clean; gix `=0.90.0` (gix-path 0.14.1, `env/mod.rs:268` lists `ucrt64`) in all six consumers |
| R1-F1.1 | worktree `launcher_layout` and Git command construction: mingw64/ucrt64 × launcher/direct, real init/status keep working | clean | resolver code unchanged; rig: real `git init`/`git status` exit 0 / `?? a.txt` in all four shapes |
| R1-F1.2 | worktree `entries::through_the_launcher_the_named_installation_is_the_prefix_git_reports`: ucrt64 launcher gix prefix = `root/ucrt64`; mingw64 control | fixed | rig `git::program` 35/35 on ucrt64 launcher (was 33/35) and mingw64; assertion tightened to require no Git subprocess (empty trace2 dir); negative control (`EXEPATH` broken) fails it |
| R1-F1.3 | worktree `entries::through_the_direct_binary_the_named_installation_is_the_prefix_git_reports`: same for direct entry | fixed | rig 35/35 on ucrt64 direct (was 33/35) and mingw64 direct; negative control fails it |
| R1-F1.4 | git-wt-cli `a_listing_never_asks_git_for_its_exec_path`: no exec-path query on ucrt64 ambient entry | fixed | rig `list_output` 10/10 on ucrt64 (was 7/10); now also rejects gix 0.90's `--show-origin` discovery query; negative control fails it |
| R1-F1.5 | git-wt-cli `a_listing_through_the_launcher_never_asks_git_for_its_exec_path`: same, launcher | fixed | rig 10/10 ucrt64 launcher + mingw64 control; negative control fails it |
| R1-F1.6 | git-wt-cli `a_listing_through_the_direct_binary_never_asks_git_for_its_exec_path`: same, direct | fixed | rig 10/10 ucrt64 direct + mingw64 control; negative control fails it |
| R1-F1.7 | worktree direct/launcher environment handling and child hint removal preserved | clean | `launcher_parity`, `direct_parity`, `the_launcher_entry_sees_the_launchers_environment`, `the_direct_entry_sees_what_the_found_binary_sees` pass in all four shapes; `forget_installation_hint` unchanged; `EXEPATH`-absent-in-children assertion kept |
| R1-F1.8 | every gix consumer compiles and keeps behavior where error handling changes | fixed | `cargo check --workspace --all-targets` exit 0; 15 migrated sites (biscuit-file `find_git_root` + error type; sniff discovery/open/merge_conflicts/remote_refresh/worktree/tracked_state/working_state/collect; worktree git_metadata/commits) with new tests `find_git_root_reports_an_unreadable_repository`, `…_treats_a_non_hex_name_as_absent`, `…_surfaces_a_prefix_of_unverifiable_length`, `unrelated_histories_are_not_ancestors`; `just lint` + `just test` pass in all six areas; Windows: sniff 2503/2503, biscuit-file 1212/1212, worktree 877/877, git-wt-cli 569 pass, darkmatter gix-affected 118/118 (scratch clone) |
| R1-F1.9 | earlier mingw64 rig receipts are invalidated by the changed inputs | fixed | receipt key hashes the committed closure + `Cargo.lock`; against the working tree worktree `cc94fa0a…` → `935ce685…`, git-wt-cli `fbd34dd4…` → `1d063706…`; no new observation published for the uncommitted tree |
| R1-F1.10 | sibling: other Git-for-Windows prefix interpretation in the six packages | clean | only `worktree` `PREFIXES` (now the same four as gix-path) and sniff `platform_extra_system_config_path` (top-level `etc`, prefix-independent) |
| R1-F2   | cold compiler tests still exceed the L1 time budget | fixed | every child row fixed or clean; no model_id test starts Cargo |
| R1-F2.1 | model_id pass and compile-fail checks: all six kept, each L1 test ≤ 5 s on a cold archive consumer | fixed | 4 compiled pass fixtures + 2 in-process diagnostics compared exactly to `.stderr`; cold consumer slowest test: macOS 0.018 s, build-win 0.045 s (fresh Cargo/rustup homes, offline, no `cargo` on PATH) |
| R1-F2.2 | same on WSL2 archive extraction and on macOS | fixed | `just cross-check model_id --os wsl` (archive mode, builder target hidden) slowest 0.008 s; macOS cold 0.018 s; WSL2 cell now executes instead of an accepted gap |
| R1-F2.3 | warm control | clean | `just test` in unchained-ai 374/374, none at or above 5 s |
| R1-F2.4 | Cargo absent still fails explicitly, never silently passes | fixed | no test needs Cargo; a pass fixture that stops compiling fails the test binary's build, a diagnostic change fails `assert_eq!`; mutation checks fail as expected |
| R1-F2.5 | offline failure before fetch and success after fetch preserved | fixed | nothing reads the registry at test time; empty Cargo home + offline passes on macOS and Windows; the fetch step and its contract exception are removed, and `ci_workflow_contracts` 221/221 with the stricter rule |
| R1-F2.6 | guard against adding an unselected fixture | fixed | `every_compile_pass_fixture_is_compiled` and `every_compile_fail_fixture_has_a_test_and_an_expected_diagnostic`, each shown to fail on an unlisted file |
| R1-F2.7 | sibling: `schematic/runtime` `slow_compile_fail_cases_report_their_intended_diagnostic` | clean | `slow_` tier, never selected by L1, so the L1 budget does not apply; tier unchanged by this fix; 29.2 s cold / 1.5 s warm |
| R1-F2.8 | sibling: `schematic/schema` `slow_generated_pager_surface_reports_the_intended_diagnostics` | clean | `slow_` tier and crate excluded from the workspace; 140.1 s cold / 9.7 s warm |
| R1-F2.9 | sibling: unchained-ai `just lint` omitted `model_id` | fixed | added `model_id` to `lint` and `lint-fix`; `just lint` exit 0 |

### Progress

- starting the work on 'R1-F1 ucrt64 Git installation discovery remains broken in gix' at 17:46:00
        - defect class: a Git installation identity that this repository's resolver recognizes is not recognized by the configuration-discovery dependency it is handed to, so discovery falls back to a subprocess and derives a different prefix
        - sweep scope: all six packages pinning `gix = "=0.84.0"` (sniff, darkmatter, biscuit-file, content-policy, simplified-schema, worktree), every typed gix error reference in them, and every consumer of worktree's `name_installation_for_gix` hint
        - delegated to a foreground rust-developer subagent; its log items are grouped below when it returns
        - subagent (rust-developer) log items for R1-F1:
                - 17:50 newest gix is 0.90.0; all six `=0.84.0` pins bumped to `=0.90.0`, `cargo update -p gix` (gix-path 0.12.1 → 0.14.1); one gix left in `Cargo.lock`
                - 17:52 confirmed `gix-path-0.14.1/src/env/mod.rs:268`: `let mut candidates = ["clangarm64", "ucrt64", "mingw64", "mingw32"]`
                - 17:55–18:00 migrated typed gix errors in biscuit-file, sniff (29 compile errors), worktree (3); darkmatter, content-policy, simplified-schema had no gix code outside tests
                - 18:00 gix-path 0.14 discovers via `git config -lz --show-origin`, and its `--exec-path` fallback now derives the right prefix, which made the identity and no-subprocess tests vacuous; both were tightened (empty `GIT_TRACE2_EVENT` dir while gix resolves the prefix; `list_output` also rejects `--show-origin`)
                - 18:02 one expectation changed: file/directory conflict naming `node` → `node~HEAD`, verified against real `git merge` + `git ls-files --unmerged` (sniff `conflict_paths_are_direction_sensitive`, darkmatter `predict_conflicts.rs`)
                - 18:13 the old rig spike dir `B:\coding\a3-spike-s5` was gone; PortableGit 2.56.0.windows.4 (ucrt64, SHA-256 matched release notes) recreated at `B:\scratch\r1f1\portable-git-2.56\git`
                - 18:17 `cargo check --workspace --all-targets` exit 0, no warnings (5m06s)
                - 18:19 `just cross-check worktree --os windows` (mingw64 rig) 877/877; ~18:31 `just cross-check git-wt-cli --os windows` 569 passed, 43 skipped
                - 18:33–19:04 blocked then resolved: PowerShell 7 `Start-Process -Wait` waits on `vctip.exe` left in cargo's job; switched to `WaitForExit()` and recorded the trap in the os skill
                - 19:05 four-shape rig matrix all green (private clone `B:\scratch\r1f1\rb`, tree `8f75a8f61` = working tree):
                        - mingw64 launcher / direct: init+status ok, `git::program` 35/35, `list_output` 10/10
                        - ucrt64 launcher / direct: init+status ok, `git::program` **35/35** (was 33/35), `list_output` **10/10** (was 7/10)
                - 19:06 negative control (`EXEPATH` pointed at temp dir in the rig clone only) failed exactly the five ledger tests; clone restored
        - decisions recorded for author review (also added to the review's `human_review_items`):
                - gix 0.90.0 rather than 0.89.x (newest; 0.89 would also fix ucrt64 with a smaller API change)
                - `is_ancestor` with no merge base now returns `Ok(false)` instead of `Err` (matches `git merge-base --is-ancestor`; the old error was a side effect of gix's typed `NotFound`)
                - merge-prediction expectation `node` → `node~HEAD` to match canonical Git
                - sniff `resolve_single_opt` length check now done locally (`prefix_hex_len_is_valid`) because gix-hash 0.28 lost typed `HexError` variants
- work completed for 'R1-F1 ucrt64 Git installation discovery remains broken in gix' at 19:07:30
- starting the work on 'R1-F2 cold compiler tests still exceed the L1 time budget' at 19:08:30
        - defect class: a single L1 test aggregates a whole set of nested compiler invocations, so its cold cost is the sum of every case plus the shared dependency build
        - sweep scope: every test in the workspace that drives `trybuild` or nested `cargo` builds (grep `trybuild::TestCases`, `Command::new("cargo")` in tests)
        - subagent (rust-developer) log items for R1-F2:
                - 19:09 measured before change (macOS, 10 cores): `ui` 9.167 s cold, 3.324 s warm
                - 19:09 nested-cargo breakdown (cold total 9.76 s): 2.80 s dependency build (syn full, proc-macro2, quote, model_id); 2.25 s second syn build (trybuild uses different flags for per-case builds); ~0.75 s per pass case; ~0.4 s per fail case; ~0.3 s fixed probes
                - 19:13 one compile-fail case alone under trybuild: 4.226 s cold, so one-test-per-case cannot meet the budget (and split tests serialize on trybuild's shared project lock); dropping syn `full` saves only ~0.4 s
                - 19:15 implemented design (b): no nested Cargo
                        - compile-pass fixtures are modules of a new `tests/derive.rs` test binary (`compile_pass_cases!` macro), so the archive build compiles each derive through the real compiler and the tests assert the generated API's behavior
                        - compile-fail fixtures run the derive's new private `expand` function in-process; the test renders the `syn::Error` as rustc prints it (message, `file:line:column`, source line, carets) and compares it exactly with the existing `.stderr` file
                        - guards: `every_compile_pass_fixture_is_compiled` and `every_compile_fail_fixture_has_a_test_and_an_expected_diagnostic`
                - 19:15 mutation checks: altered `.stderr` column fails `bad_attribute`; altered derive message fails `not_enum`; unlisted fail/pass fixtures fail their guards
                - 19:16 a temporary real-trybuild run of `tests/fail/*.rs` against the changed macro passed, so the `.stderr` files still equal rustc's real output; probe removed
                - 19:17–19:21 `just lint` (unchained-ai) pass; clippy `-D warnings` clean; test-toolkit `ci_workflow_contracts` 221/221; `just check-tier-coverage unchained-ai` nothing stranded; resolved plan now schedules model_id's wsl2-ubuntu L1 cell instead of an accepted gap
                - 19:19 macOS cold consumer (fresh archive extraction, empty Cargo home, offline, no `cargo` on PATH): 9/9, slowest 0.018 s
                - 19:26 `just cross-check model_id --os windows`: pass, slowest 0.048 s
                - 19:28 build-win cold consumer (fresh archive, fresh Cargo/rustup homes, offline, no `cargo` on PATH, private scratch dir deleted after): 9/9, slowest 0.045 s
                - 19:28 `just cross-check model_id --os wsl` (archive mode, builder target hidden): pass, slowest 0.008 s
                - removed: `trybuild` dev-dependency, model_id's `requires-toolchain`, the model_id `cargo fetch --locked` workflow step, and the matching `ci_workflow_contracts` exception (the general `requires-toolchain` machinery stays for test-toolkit and repo-deps)
        - sibling sweep: `schematic/runtime` `slow_compile_fail_cases_report_their_intended_diagnostic` (29.2 s cold) and `schematic/schema` `slow_generated_pager_surface_reports_the_intended_diagnostics` (140.1 s cold; the crate is excluded from the workspace) are both `slow_`-tier, so the L1 filter never selects them and the 5 s L1 budget does not apply; this fix did not change their tier, so both are **clean** for this class. The subagent proposed "disputed"; the orchestrator overrode that because the class (an L1 test over budget) does not include them. `schematic/reviews/**/spikes` crates are not workspace members
        - orchestrator: the subagent noted unchained-ai's `just lint` omitted `model_id` although `just test` runs it; added `model_id` to `lint` and `lint-fix` in `unchained-ai/justfile` (Rule 5)
        - decisions recorded for author review (also added to the review's `human_review_items`):
                - the trybuild suite was replaced by in-process diagnostics plus compiled pass fixtures; the remaining unexercised layer is `into_compile_error` plus rustc's own rendering, and `unchained-ai/docs/testing.md` says to recheck by hand when error reporting changes
                - removing `requires-toolchain` turns model_id's WSL2 cell from an accepted gap into an executing cell (coverage widens; the capability policy itself is unchanged)
- work completed for 'R1-F2 cold compiler tests still exceed the L1 time budget' at 19:32:01
- 19:34–19:55 orchestrator verification (macOS, serial, no subagent running):
        - `just lint` + `just test` per area: unchained-ai 374/374; biscuit-file 1286/1286; content-policy 176/176; simplified-schema 1842/1842; worktree 1510/1510; sniff 3565/3565; darkmatter 16370/16370 — all exit 0
        - `just test test-toolkit` 526/526 and `cargo clippy -p test-toolkit --all-targets -D warnings` clean (the test-toolkit directory has no area `lint`/`test` recipes)
        - `just check-tier-coverage` for sniff, biscuit-file, worktree, darkmatter, unchained-ai: exit 0
        - slow tests seen, all pre-existing and unrelated to either finding: darkmatter-cli `claudine_schema_bundle::{depth_four_container_controls…, malformed_identifying_values…}` (5.2–5.4 s, also alone) and test-toolkit `archive_path_guard`/`ci_workflow_contracts`/`kache_host_contracts` (5.2–8.0 s); the timing history records the same tests at similar durations at `dabb24107`, `1a2c5429a`, and `af7ec2d63`, before this cycle
        - Windows cross-checks: sniff 2503/2503, biscuit-file 1212/1212 (plus worktree, git-wt-cli, and model_id from the subagents); `just cross-check darkmatter --os windows` built its archive but was refused by the rig's storage preflight (48.5 GiB free, 50 GiB required, sweep freed 0). The orchestrator removed only this cycle's own scratch `target` (5.8 GiB) and then ran the gix-affected darkmatter tests natively in the private scratch clone: 118/118; the scratch `target` was removed afterward

### Completion Report

- outcome: **complete**; elapsed about 2 h 11 min (17:44:54 → 19:56)
- parent findings: total 2 = fixed 2 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- verified repairs:
        - R1-F1: gix `=0.84.0` → `=0.90.0` in sniff, darkmatter, biscuit-file, content-policy, simplified-schema, worktree; error-API migration at 15 sites with behavior-preserving tests; ucrt64 launcher and direct entry pass on native Windows with PortableGit 2.56.0.windows.4, mingw64 controls pass, and a negative control proves the tightened tests detect an ignored hint
        - R1-F2: model_id has no nested Cargo; four pass fixtures compile into `tests/derive.rs`, two diagnostics are checked in-process against their `.stderr` files, and two guards protect fixture selection; the slowest cold-consumer test is 0.045 s on Windows
- open finding or child IDs: none
- remaining limitations (not findings): the full darkmatter Windows cross-check was refused by rig disk headroom (scoped native run passed); hosted ucrt64 confirmation still comes from a hosted run, because rig receipts come from a mingw64 host; three-nightly confirmation stays with the scheduled runs
- changed files:
        - R1-F1: `Cargo.lock`; `{sniff,darkmatter,biscuit-file,content-policy,simplified-schema,worktree}/lib/Cargo.toml`; `biscuit-file/lib/src/file_reference/{context.rs,error.rs}`; `sniff/lib/src/filesystem/git/{discovery.rs,open.rs,merge_conflicts.rs,remote_refresh.rs,worktree.rs,tracked_state.rs,working_state.rs,recent_commits/collect.rs}`; `sniff/lib/tests/l1/{git_parity.rs,merge_conflict_prediction.rs}`; `darkmatter/lib/tests/l1/predict_conflicts.rs`; `worktree/lib/src/git/program.rs`; `worktree/lib/src/git_metadata.rs`; `worktree/lib/src/graph/topology/commits.rs`; `worktree/cli/tests/list_output.rs`; docs `docs/dependencies.md`, `sniff/docs/dependencies.md`, `content-policy/docs/dependencies.md`, `sniff/lib/README.md`, `worktree/docs/git-for-windows.md`, `worktree/docs/performance-testing.md`, `.claude/skills/os/{SKILL.md,windows.md}`, `.claude/skills/worktree/SKILL.md`
        - R1-F2: `unchained-ai/model_id/{Cargo.toml,src/lib.rs}`; `unchained-ai/model_id/tests/derive.rs` (new); `unchained-ai/model_id/tests/ui.rs` (deleted); `unchained-ai/model_id/tests/pass/*.rs`; `Cargo.lock`; `.github/workflows/_package-ci.yml`; `tools/test-toolkit/tests/ci_workflow_contracts.rs`; `unchained-ai/docs/testing.md`; `unchained-ai/justfile`; `.claude/skills/os/wsl.md`; `docs/dependencies.md`
- final verification commands: listed in the 19:34–19:55 entry above (R1-F1.8 and R1-F2.1–F2.6) and in each subagent group (R1-F1.1–F1.7, R1-F1.9, R1-F2.1–F2.2)
