---
implementation_1: "2026-11-21T20:12:18+00:00"
---

# Commit link walks: review/fix log

## Implementation of Review Findings #1

> **started at:** 2026-11-21T20:12:18+00:00

- this implementation is attempting to implement _all_ of the review findings found in '/home/build/wt/rusty-biscuit/fix-biscuit-terminal/sniff/fixes/2026-11-20-commit-link-walks/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- review frontmatter `implemented` confirmed `false` before repairs started
- review lists no `human_in_the_loop` items, no blocked findings, and no recurrence
- Native Windows evidence for this repair was obtained on `build-win`; macOS and WSL2 runs were left to CI (see the ledger and completion report).

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1   | Required Windows Git startup optimization is missing | fixed | all five children verified; `just cross-check` on `build-win` passed sniff 38/38 and sniff-detection-cli 10/10 (filtered, patched tree) |
| R1-F1.1 | Installation hint helpers and installation lookup owned by Sniff's program lookup | fixed | `sniff/lib/src/programs/git_program.rs` now owns `command`, `SET_EXEPATH`, `name_installation_for_gix`, `forget_installation_hint`, and the installation lookup; `worktree/lib/src/git/program.rs` deleted. Decision: `command()` moved too, since it shares the lookup; alternative (private copy or public internals) would duplicate the lookup |
| R1-F1.2 | Sniff main binary sets the hint from a plain `main` before the Tokio runtime starts | fixed | `sniff/cli/src/main.rs` plain `fn main` sets the hint, then builds a multi-thread runtime; Windows test `git_installation::a_repository_run_without_an_installation_hint_starts_no_git_process` (zero trace2 files) plus control that does start Git; passed on `build-win` |
| R1-F1.3 | Sniff subprocess chokepoint removes only an internally supplied hint from children | fixed | hook at `sniff/lib/src/process.rs:340` in `run_command_with_timeout`; sweep found every production spawn routes through it; tests `a_child_does_not_inherit_the_installation_hint_sniff_set` / `a_child_inherits_the_installation_hint_its_caller_set` (fail with hook removed) |
| R1-F1.4 | Worktree startup and command helper switch to the shared helpers, keeping behavior | fixed | `worktree/lib/src/git.rs:10` re-exports sniff copies; `wt` call sites unchanged; `worktree --lib` 870 passed, list_output/refresh_worker 53 passed |
| R1-F1.5 | Other Sniff binaries: none opens a repository at startup | clean | render fixtures build synthetic `GitInfo`/`CiCdInfo`; `foreground_handoff_fixture` spawns a shell and opens no repository; `sniff/lib` declares no `[[bin]]` |
| R1-F2   | Root-package status selection hides history and changes | fixed | all eight children verified; `reproduce.py` R1-F2 rows match expected; lib + CLI scoped runs 97 + 172 passed |
| R1-F2.1 | Package-prefix resolver yields empty prefix for root package; history retained | fixed | `scope.rs` `directory_prefix` maps root `""`/`.`/`./` to `""`; `scope::tests::a_root_package_resolves_to_the_empty_prefix` and `status_report::tests::a_root_package_selection_keeps_the_repository_history_and_changes` (both red with fix reverted). Decision: root selection spans the whole repository (reviewer’s repair; matches `dirty-files`); alternative exclusive root attribution would need exclusion support in the path-history reader; flagged in review `human_review_items` |
| R1-F2.2 | Working-tree narrowing and recount retain root changes; dirty reported | fixed | same status test asserts root Both/Untracked + member Modified, counts (1,2,1), `is_dirty`; CLI `test_repo_git_status_root_package_keeps_history_and_changes` |
| R1-F2.3 | Final link selection keeps displayed root commit eligible for its link | fixed | status test asserts link keys equal the full ID of the displayed root commit; compact text shows the linked root commit |
| R1-F2.4 | Nested package selection (`-p member`) control | clean | `-p member` → `"area/member/"` (`a_nested_package_or_area_resolves_to_a_slash_terminated_prefix`); status control gives member change only, (0,1,0), linked |
| R1-F2.5 | Area selection and package/area intersection controls | clean | area resolver filters empty areas (`scope.rs:93-97`); `-a area` / `-p member -a area` member-only; `-p root-app -a area` errors “in area ” (`a_root_package_never_intersects_a_named_area`) |
| R1-F2.6 | Later branch override control | clean | branch override replaces `recent` and clears working tree; reproduction: recent 1, no changes, clean |
| R1-F2.7 | Package and area catalog handlers control | clean | `commands/repo.rs:130,219` discard the prefix and filter by identity; reproduction `['root-app']`, `['']` |
| R1-F2.8 | Remaining generic validation consumers discard the prefix | clean | `commands/mod.rs:1498` discards the result; only prefix consumer is `status_report.rs:109` |
| R1-F3   | Dirty package reports read arrays absent from ordinary requests | fixed | all nine children verified; `reproduce.py` every row as expected; CLI scoped 380 passed, lib scoped 118 passed |
| R1-F3.1 | Dirty package collector / JSON projection names changed packages | fixed | library `observed_scope_paths` (`aggregate_view.rs`) reads `file_changes`; CLI `changed_package_names` → `attribute_paths`; `cli::dirty_package_families_name_packages_from_ordinary_file_changes` (red before: `left: []`); lib `l1::aggregate_changes` collects status the ordinary way and asserts empty `status.dirty`/`untracked` |
| R1-F3.2 | Dirty area projection names changed areas (`''` root, `area` member) | fixed | areas derived from fixed names; script `[""]` root / `["area"]` member; same test asserts `["", "area"]` unscoped |
| R1-F3.3 | Dirty package and area text renderers show names, succeed | fixed | `dirty-packages -p member` prints `member`, `dirty-package-areas -p member` prints `area`, both exit 0 (script + test) |
| R1-F3.4 | Staged package/area projections control | clean | reads `file_changes` with `Staged|Both`; script rows correct; R1-F3.8 boundary flaw in its copy fixed via shared `attribute_paths` |
| R1-F3.5 | Unstaged package/area projections control | clean | reads `file_changes` with `Modified|Both`; script rows correct; R1-F3.8 boundary flaw fixed. Observation: unstaged command still excludes conflicted files while `ChangeScope::Unstaged` includes them; existing behavior, kept |
| R1-F3.6 | Current-area dirty and source-change decisions control | fixed | no missing changes, but sweep found area string-prefix check (`area-a` claimed `area-ab/…`) and deep-mode double count; both use `area_changed_paths`; tests `current_package_area_ignores_a_sibling_sharing_the_area_prefix`, `package_area_source_code_change_count_counts_source_files_only` |
| R1-F3.7 | Dirty package/area clean controls stay empty | clean | after reset all four commands return empty `names` (script + test) |
| R1-F3.9 | Sweep sibling: `-v` filesystem listing read patch-only arrays, hiding dirty/untracked files on ordinary requests | fixed | `cli/src/output/filesystem/repo.rs` lists from `file_changes`; `verbose_filesystem_section_lists_ordinary_file_changes` |
| R1-F3.8 | Discovered in R1-F2 sweep: package ownership `starts_with(relative)` lacks a path-segment boundary (`sniff/lib/src/.../aggregate.rs:816`, CLI `packages.rs`) | fixed | `path_is_within` in `aggregate.rs` used by both branches of `package_contains_path`; three CLI copies removed; `cli::dirty_packages_respect_package_directory_boundaries` (red before: `["member"]` for `area/member2/notes.rs`), `attribute_paths_respects_path_segment_boundaries`, `attribute_paths_gives_the_root_package_only_unclaimed_paths` |

### Progress

- starting the work on 'R1-F1: Required Windows Git startup optimization is missing' at 20:12:52
        - defect class: a process-wide environment hint that a dependency reads at first repository open is set by one consumer binary only, so other binaries pay the Git child-process cost and children could inherit an internally set hint
        - delegated to a foreground `rust-developer` subagent; its log items are recorded below on return
        - moved the helper, flag, installation lookup, and `command()` into `sniff::programs::git_program`; `wt` re-exports all three functions so its public API is unchanged
        - `sniff` CLI now uses a plain `fn main` that sets the hint before building the multi-thread Tokio runtime
        - the hook runs once at `run_command_with_timeout`; child-spawn sweep:
                - every service, network, hardware, version-probe, and OS probe uses `run_with_timeout` → chokepoint
                - `git fetch` (remote refresh) and all install/uv commands use the chokepoint as their runner
                - `ProcessTree`, `ForegroundHandoff`, `configure_detached_child` spawn nothing themselves
                - Git child processes started by `gix` itself (filter drivers) still inherit the hint, as in `wt` before
        - discovery: the CLI test fixture sets `GIT_CONFIG_NOSYSTEM=1`, which may skip gix's installation lookup; the new test removes it and pairs with a control run that does start Git
        - no Windows cross-compile target is installed locally; Windows evidence came from filtered `just cross-check` runs on `build-win` (sniff 38/38, sniff-detection-cli 10/10); no published observation because the tree is uncommitted
        - focused checks: sniff lib 2136 passed (`--features remote --lib`), worktree lib 870 passed, clippy `-D warnings` clean for sniff, sniff-detection-cli, worktree, git-wt-cli
        - docs updated: `sniff/docs/sniff-library-architecture.md`, `worktree/docs/git-for-windows.md`, sniff/worktree/os skills
- work completed for 'R1-F1: Required Windows Git startup optimization is missing' at 20:44:02
- starting the work on 'R1-F2: Root-package status selection hides history and changes' at 20:44:02
        - defect class: an empty repository-root relative path is normalized into a non-empty separator-prefixed scope, so every repository-relative path filter that uses it rejects valid paths
        - delegated to a foreground `rust-developer` subagent
        - fix: `resolve_package_path` now uses `directory_prefix`, which returns `""` for every root spelling and `dir/` otherwise; Cargo, pnpm, and npm detectors all record root `relative` as `""`
        - decision: root `-p` selects the whole repository, including nested members (the reviewer's repair; matches `dirty-files --package`); `recent-commits --package` and aggregate attribution instead give the root only unclaimed paths; an exclusive root would need exclusion support in the path-history reader; recorded for author review
        - sibling sweep: area prefix, narrowing, path history, aggregate/CLI root handling, package areas, ownership, blast radius, committed tree all clean for an empty root; `remote/snapshot.rs:71` and the docs area filter are not this class
        - discovery: nested-package ownership uses `starts_with(relative)` with no segment boundary (`aggregate.rs:816`, CLI `packages.rs`), so `area/member` also claims `area/member2/…`; added as R1-F3.8 because it sits in the R1-F3 projection code
        - discovery (cosmetic, not a defect against the spec): root `Package.path` serializes with a trailing `/.`
        - tests: 4 resolver unit tests, 1 public status-preparation regression, 1 spawned-CLI wiring test; all under 0.1 s
        - docs: `sniff/docs/cli/repo_git-status.md` states root selection behavior; sniff architecture skill notes the prefix rule
- work completed for 'R1-F2: Root-package status selection hides history and changes' at 20:51:46
- starting the work on 'R1-F3: Dirty package reports read arrays absent from ordinary requests' at 20:51:46
        - defect class: a projection reads optional detail data that the request did not ask for, so an unrequested empty array is taken as evidence that the underlying set is empty
        - delegated to a foreground `rust-developer` subagent, together with discovered row R1-F3.8
        - fix: new library helpers `observed_scope_paths` and `area_changed_paths` read changed paths from `file_changes`, returning `None` (unknown) for identity-only status or dirty status without file changes; patch collection not enabled
        - CLI dirty/staged/unstaged package projections share one `changed_package_names` helper over the library's `attribute_paths`
        - R1-F3.8: ownership is now path-segment aware (`path_is_within`), including the root's unclaimed-path check
        - sweep found and fixed three more defects:
                - CLI current-area dirty/source-change helpers used a string prefix (`area-a` claimed `area-ab/…`)
                - deep (`--refresh-remotes`) mode counted a source file twice
                - `sniff -v` filesystem listing hid dirty and untracked files on ordinary requests (R1-F3.9)
        - sweep clean: status narrowing, `changed_paths`, status counts line, worktree section, `repo_json` buckets, blast radius, scope intersection, remote snapshot, docs filter; `--package-area` remains a documented case-insensitive prefix match on user input
        - unit-test fixtures that set `status.dirty` without `file_changes` (a state the library never produces) were rewritten
        - docs: `repo_dirty-packages.md`, `repo_dirty-package-areas.md`, sniff skill invariant
        - red-before-green run for the two CLI wiring tests and the boundary test; reasoned only for three new CLI unit tests
- work completed for 'R1-F3: Dirty package reports read arrays absent from ordinary requests' at 21:07:00
- starting the phase gates (`just test`, `just lint` in `sniff/` and `worktree/`, strict Clippy) at 21:07:00
        - `sniff`: `just test` 3879 passed, 35 skipped, none at or above 5.0 s; `just lint` exit 0
        - `worktree`: `just test` 1480 passed, 13 skipped, none at or above 5.0 s; `just lint` exit 0
        - `cargo clippy -p sniff -p sniff-detection-cli -p worktree --all-targets -- -D warnings` clean; same for `-p sniff --features remote`
        - `just check-tier-coverage sniff` / `worktree`: 0 stranded
        - fresh build + `python3 -I target/commit-link-walks-review-1/reproduce.py`: every R1-F2 and R1-F3 row matches its expected result, including clean controls
- phase gates completed at 21:10:52

### Completion Report

- **outcome: complete**; elapsed about 58 minutes (20:12:18 → 21:10:52 UTC)
- parent findings: total 3 = fixed 3 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- instance rows: 22 (5 R1-F1, 8 R1-F2, 9 R1-F3 including discovered R1-F3.8 and R1-F3.9); none open
- verified repairs:
        - R1-F1: helper, flag, installation lookup and `command()` moved into `sniff::programs::git_program`; `sniff` sets the hint from a plain `fn main` before the Tokio runtime; chokepoint removes only the internally set hint; `wt` re-exports the moved copy
        - R1-F2: root package resolves to the empty prefix; status history, changes, counts, and links are retained
        - R1-F3: dirty projections read `file_changes` through library helpers; ownership is segment-aware; three sibling defects fixed (area string prefix, deep-mode double count, `-v` listing)
- decision recorded for author review: root `-p` selects the whole repository in `git-status` (matching `dirty-files`), while `recent-commits` and package attribution give the root only unclaimed paths
- observation, unchanged existing behavior: `unstaged-packages` excludes conflicted files while the library's unstaged scope includes them
- OS evidence: Linux local; native Windows via filtered `just cross-check` on `build-win` for R1-F1 (sniff 38/38, sniff-detection-cli 10/10), no published observation because the tree is uncommitted; macOS and WSL2 left to CI.
- no performance measurement was required by these findings; `deferred_perf_measurement` not set
- changed files:
        - R1-F1: `sniff/lib/src/programs/git_program.rs` (new), `sniff/lib/src/programs/mod.rs`, `sniff/lib/src/process.rs`, `sniff/cli/src/main.rs`, `sniff/cli/tests/l1/git_installation.rs` (new), `sniff/cli/tests/l1/main.rs`, `worktree/lib/src/git.rs`, `worktree/lib/src/git/program.rs` (deleted), `sniff/docs/sniff-library-architecture.md`, `worktree/docs/git-for-windows.md`, `.claude/skills/os/windows.md`, `.claude/skills/worktree/SKILL.md`
        - R1-F2: `sniff/lib/src/filesystem/repo/scope.rs`, `sniff/lib/src/filesystem/git/status_report_tests.rs`, `sniff/docs/cli/repo_git-status.md`
        - R1-F3: `sniff/lib/src/filesystem/repo/aggregate.rs`, `aggregate_view.rs`, `repo/mod.rs`, `sniff/lib/tests/l1/aggregate_changes.rs` (new), `sniff/lib/tests/l1/main.rs`, `sniff/cli/src/output/filesystem/{mod,packages,repo}.rs`, `sniff/cli/src/output/repo_json.rs`, `sniff/docs/cli/repo_dirty-packages.md`, `repo_dirty-package-areas.md`, `.claude/skills/sniff/SKILL.md`
        - shared: `sniff/cli/tests/l1/cli.rs` (R1-F2, R1-F3), `.claude/skills/sniff/architecture.md` (R1-F1, R1-F2)
- nothing committed; the author's existing edits to the plan, spec, and implementation log were left untouched

### Review correction — 2026-11-21

The opening and completion statements that no OS testing rigs were configured
for this task contradicted this record's Windows execution on `build-win`.
They now describe the evidence actually recorded. Review 2 also directly
checked the environment: `BUILD_WIN=build-win`, `BUILD_WSL=build-wsl`, and
`BUILD_MACOS=bolt` are set. That check establishes current configuration,
not which variables were set during earlier implementation phases; their
historical environment claims have not been rewritten.

Counting the ledger's child IDs also showed that R1-F3 has nine instances,
not eight as its summary row said or ten as the completion report said.
Those counts are corrected; the total is 22 child rows, not 23.
