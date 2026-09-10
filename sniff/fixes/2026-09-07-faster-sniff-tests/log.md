---
implementation_1: 2026-09-09T15:04:39-07:00
deferred_perf_measurement: false
---

# Validation log

## Standing coordination rule

No timing or work-count comparison may span the landing of
`2026-07-22-inefficient-calling`. Every reading must mark the
`production-caching-2026-07-22` boundary as `before` or `after`. If the fix
lands during this work, affected cohorts must be re-baselined at the new
revision before candidate comparisons continue.

## Phase 1

All local Rust measurements below used the clean detached source and preserved
build directory recorded in
[`baseline/local-c2dee9217/metadata.md`](baseline/local-c2dee9217/metadata.md).
The host was native arm64 macOS 27.0. No source changed between the warm build
and the measured L1, sanity, L2, and counter runs.

| Command | Selection and profile | Result | Build/setup | Runner elapsed | Summed duration | Artifact |
|---|---|---:|---:|---:|---:|---|
| `tools/test-audit: just check` | TypeScript typecheck + Vitest | 164 passed | 3.55 s Vitest setup/collection | 16.89 s Vitest wall | 19.93 s test sum | [`phase1-test-audit-check.log`](phase1-test-audit-check.log) |
| `config validate` | Sniff audit config | pass | — | <1 s | — | [`phase1-config-validate.log`](phase1-config-validate.log) |
| `capture` | Six declared feature selections | 6 passed | listing builds were contended | 1,509.9 s total listing wall | — | [`enumeration/captures.json`](enumeration/captures.json) |
| `just test` | default profile; `sniff/remote` + bare `sniff-cli`; L1 filter | 2,599 passed, 23 excluded, 0 failed | 10.88 s warm delta | 28.552 s | 439.14 s | [`just-test.log`](baseline/local-c2dee9217/just-test.log), [`just-test.xml`](baseline/local-c2dee9217/just-test.xml) |
| `just sanity` | default profile; lib/bin only; `sniff/remote` + `sniff-cli/test-fixtures` | 1,820 passed, 23 excluded, 0 failed | 122 s across distinct feature builds | 8.345 s | 112.19 s | [`just-sanity.log`](baseline/local-c2dee9217/just-sanity.log), [`sanity-junit/`](baseline/local-c2dee9217/sanity-junit/) |
| `just test-l2` | default profile; `sniff-cli/test-fixtures`; tmux required | 2 passed, 790 excluded, 0 failed | 2.71 s | 1.410 s | 1.410 s | [`just-test-l2.log`](baseline/local-c2dee9217/just-test-l2.log), [`l2-junit/`](baseline/local-c2dee9217/l2-junit/) |
| `work_counts` | `staged_filesystem_full_all_stages` | pass | 10.06 s | 44.4 ms directional | — | [`work-counts.stdout`](baseline/local-c2dee9217/work-counts.stdout), [`work-counts.json`](baseline/local-c2dee9217/work-counts.json) |
| `counters validate` | Eight configured signal readings | pass | — | <1 s | — | [`work-counts.validate.txt`](baseline/local-c2dee9217/work-counts.validate.txt) |
| `counters compare` | Baseline against identical provenance control | pass, eight zero deltas | — | <1 s | — | [`work-counts.compare-control.md`](baseline/local-c2dee9217/work-counts.compare-control.md) |
| `measure report` | L1, sanity, and L2 local manifests | pass | separate per run | separate per run | separate per run | [`measure-report.md`](baseline/local-c2dee9217/measure-report.md) |
| `junit` | CI run 34008778001, all declared native/WSL cells | pass | reported per cell | reported per cell | reported per cell | [`junit-gate.md`](baseline/34008778001/junit-gate.md) |
| `just lint` | `sniff/remote` + `sniff-cli` | pass, no warnings | 57.63 s | — | — | [`just-lint.log`](baseline/local-c2dee9217/just-lint.log) |

The L1 console emitted seven slow notices for six distinct tests: the two
system-view cap tests, three bench-fixture tests, and
`test_detect_completes_in_reasonable_time`; one bench-fixture test crossed both
the 5-second and 10-second notice thresholds. There were no failures, retries,
timeouts, or leaks. The sanity cohort's combined 8.345-second runner elapsed is
within its 15-second budget. The L2 backend ledger records two `tmux` `run`
decisions, so installed-but-unexercised tmux could not satisfy the gate.

The first attempted sanity command incorrectly supplied `--config-file` as a
`just` argument and failed before compiling or running tests. The successful
rerun used the supported `BISCUIT_NEXTEST_BIN` injection; no test evidence was
lost or retried.

## Phase 2

Phase 2 changed only audit evidence and documentation. No Rust behavior or
test was changed, so the requirement-to-test mapping is the audit contract:
the source scan preserves cfg/ignore metadata; the six captures cover every
canonical feature selection; and reconciliation rejects missing, duplicate,
or multiply assigned identities and undocumented exclusions. Those behaviors
are covered by the shared `tools/test-audit` suite.

| Command | Selection | Result | Evidence |
|---|---|---:|---|
| `config validate` | Sniff audit configuration | pass | six selections, two packages, four CI environments |
| `sources --json` / `--markdown` | 216 Sniff Rust files | pass | 2,665 attributes, 41 platform exclusions, 15 ignored helpers, five retained parser diagnostics |
| focused rustdoc listings | `sniff/remote`; `sniff-cli/test-fixtures` | pass | 113 library doctests discovered, zero CLI doctests |
| `just doctest` | canonical Sniff-area doctest gate | pass | library: 91 passed, 22 ignored; CLI: zero tests |
| `reconcile --markdown` | six captures, 81 families, live source scan | pass | 2,624 identities assigned exactly once; no runner-only, stale, duplicate, or undeclared rows |
| `tools/test-audit: just check` | TypeScript typecheck + Vitest | pass | 164 tests across 13 files |
| focused nextest regression check | `test_detect_completes_in_reasonable_time` | pass | 1 passed in 3.467 s |
| `just test` | Sniff package-area L1 selection | pass on warm rerun | 2,599 passed, 23 skipped, 0 failed in 62.380 s |
| `just lint` | `sniff/remote` + bare `sniff-cli` | pass | no diagnostics; finished in 1 min 20 s |

The pre-existing generated source reports were moved out of `enumeration/` to
the fix root. The shared reconciler treats every JSON file inside
`enumeration/` as a listing capture and correctly rejected the old layout as
an undeclared capture; no audit-tool implementation change was needed.

The first broad `just test` attempt ran during extreme host contention (load
averages reached 367.71/210.98/114.05) and nextest timed out the pre-existing
`test_detect_completes_in_reasonable_time` test at its 30-second runner limit;
1,760 tests had passed, 838 were canceled, and none had failed. A direct
focused rerun passed in 3.467 seconds, and the subsequent warm full-area rerun
passed all 2,599 selected tests. One attempted focused invocation through
`just test 'test(...)'` was rejected by the shell before test execution because
the recipe interpolates the filter without quoting; it contributed no test
result. The canonical full-suite rerun is the completion gate.

## Phase 3

Phase 3 changed only analysis evidence and the execution plan. No Rust source,
parser, schema, shipped template, configuration behavior, or CLI output changed,
so there is no implementation regression input or passive/end-to-end artifact
test to add. The requirement-to-test mapping is:

- family timing attribution → the shared `attribute` command joined Phase 1
  JUnit identities to the reconciled family catalog;
- reachability and feature decisions → Phase 2's six listing captures and
  zero-violation macOS reconciliation;
- work budgets → Phase 1's validated collector-propagated `work_counts`
  report plus existing aggregate CLI counter assertions;
- document consistency → `config validate`, `reconcile`, and the shared
  `tools/test-audit` typecheck/Vitest suite;
- package health → canonical Sniff-area `just test` and `just lint` completion
  gates recorded below.

### Attribution

| Command/input | Selection | Result | Evidence |
|---|---|---:|---|
| `attribute baseline/local-c2dee9217/just-test.xml --markdown/--json` | Local L1, all 2,599 selected tests | pass; 81 families; 439.14 s summed, 28.552 s runner elapsed | Phase 3 table in `inventory.md`; Phase 1 JUnit retained unchanged |
| `attribute` over run 34008778001 L1 XMLs | macOS, Ubuntu, Windows, WSL | macOS/Ubuntu/WSL pass; Windows reports two family-classification violations | Native cohort table in `inventory.md`; original JUnit remains the execution authority |
| Windows classification review | `windows_app_paths_orphan` and `windows_find_program_priority` | both passed; 0.03 s combined; listed as source-side platform exclusions but absent from macOS-derived family matching | recorded as evidence limitation, not a product defect or skipped test |

The stored CI run is one green observation per environment. The audit tool's
budget command requires at least three compatible green CI runs and refuses
local provenance, so numeric per-family CI timing budgets remain pending by
design until Phase 8. Work-budget classes, the existing 15-second sanity
budget, native cohort separation, and the Windows no-concurrency-change
decision are ratified in `inventory.md`.

### Validation

| Command | Selection | Result |
|---|---|---:|
| `config validate` | Sniff audit configuration | pass: two packages, six selections, four environments |
| `reconcile --markdown` | Six captures, 81 families, live source scan | pass: 2,624 identities, 41 declared platform exclusions, no violations |
| `tools/test-audit: just check` | TypeScript typecheck + Vitest | pass: 164 tests across 13 files |
| `just test` | Sniff package-area L1 (`sniff/remote` + bare `sniff-cli`) | pass: 2,599 passed, 23 skipped, 0 failed in 42.421 s; one 5.342 s slow notice, no timeout/retry |
| `just lint` | `sniff/remote` + bare `sniff-cli` | pass: no diagnostics |

The first attempt to invoke `config validate` and `reconcile` from the audit
tool directory also supplied `pnpm --dir tools/test-audit`, producing an
immediate path-resolution error before either command ran. The corrected
commands used `pnpm exec` from that directory and both passed; this was a
command invocation error, not a test failure.

## Phase 4

Phase 4 added only `sniff-cli` test infrastructure and migrated the existing
software-contract helper plus four parent-side Git setup sites. No production
parser, schema, template, prompt, persistence format, or configuration-driven
behavior changed, so passive shipped-artifact corpus and read/write/read tests
are not applicable.

### Requirement-to-test mapping

| Requirement | Observable proof |
|---|---|
| Assert/raw command parity and ordered policy | `assert_and_raw_surfaces_produce_the_same_effective_environment` executes an environment recorder through both adapters and compares the complete captured maps; `intentional_override_after_policy_wins_over_the_scrub` proves a post-build per-key override survives. |
| Ambient and checkout containment | `ambient_context_accepts_only_existing_fixture_directories`, `canonical_checkout_containment_rejects_nested_roots`, and the Unix symlink-spelling variant prove accepted, missing, outside, canonicalized-inside, and canonicalized-outside boundaries. |
| Bounded PATH and host/fake escapes | `path_modes_are_bounded_and_keep_fixture_stubs_first` checks fixture-first ordering, exact fake-only contents, and the minimal system-entry count. Its call-site comments name the absence proof and real `git` tool. |
| Hostile inherited inputs | `inherited_git_sniff_and_rendering_inputs_are_scrubbed` uses the original contamination shapes `GIT_DIR=/host/repository.git`, `SNIFF_WAN_IP_ENDPOINTS=http://host.invalid/ip`, `FORCE_COLOR=1`, and `XDG_CONFIG_HOME=/host/config`, then asserts Git/Sniff/color absence plus fixture config and `NO_COLOR` downstream state. |
| Real shipped CLI path | `real_sniff_command_observes_the_fixture_launch_context` initializes a disposable repository through isolated parent-side Git, invokes the built `sniff` normally, and asserts valid JSON, clean stderr, success status, and the non-monorepo projection. |
| Guard correctness and census | `detector_finds_all_raw_spawn_forms_and_ignores_prose_and_strings` covers the three forbidden forms, whitespace variants, prose/comments/raw strings, other binaries, and the sanctioned raw surface; `restored_detector_rejects_a_violation_that_a_neutered_detector_misses` records the negative mutation; `reconciliation_rejects_stale_and_unexplained_entries` proves stale/blank entries fail. The live gate counted 372 sites across exactly six allowlisted files and wrote `sniff-spawn-site-burn-down.jsonl`. |
| Migrated consumers and parent-side Git | The focused `software` run passed 33 tests; the `repo_is_monorepo_` run passed seven; `repo_json_succeeds_in_a_shallow_clone` passed independently. These retain stdout/JSON/stderr/exit status and shallow-history assertions after isolation. |

The hostile-input regression initially failed because an explicitly preloaded
`SNIFF_WAN_IP_ENDPOINTS` was not removed when that key was absent from the
parent process. Adding the shipped application input to the unconditional
scrub made the test pass; the final broad gates below include that fix.

### Validation

| Command | Selection | Result |
|---|---|---:|
| `cargo test -p sniff-cli --test cli_process_fixture --test spawn_site_guard` | fixture, drift, containment, PATH, real CLI, detector mutation, stale allowlist, live census | final broad gate includes all 12 tests; final focused fixture target passed all 8 tests |
| `cargo test -p sniff-cli --test cli software` | affected software family | pass: 33 |
| `cargo test -p sniff-cli --test cli repo_is_monorepo_` | affected monorepo predicate family | pass: 7 |
| `cargo test -p sniff-cli --test cli repo_json_succeeds_in_a_shallow_clone` | isolated parent-side Git/shallow clone | pass: 1 |
| `just test` | final Sniff-area L1 state (`sniff/remote` + bare `sniff-cli`) | pass: 2,610 passed, 24 skipped, 0 failed in 41.986 s; one pre-existing ambient-checkout test marked slow |
| `just lint` | final `sniff/remote` + `sniff-cli` state | pass: no diagnostics |

No retry, timeout, tier, or nextest override changed in Phase 4. The existing
`git diff main -- .config/nextest.toml` contains unrelated pre-existing
override cleanup, but no Phase 4 file touches that configuration. This host is
native arm64 macOS; the area has no local mingw check recipe, so compilation
and runtime verification of the Windows-only command-recording branch remains
pending on the canonical `windows-latest` CI leg. Both recorder branches are
kept in one `cfg!(windows)` function so rustc parses and type-checks both on
this host.

The required GitNexus `detect_changes` audit ran against the unstaged shared
worktree. It reported critical aggregate risk across 1,321 files because the
checkout already contains unrelated concurrent changes throughout the
monorepo; that aggregate is not Phase 4's blast radius. The Phase 4 path-scoped
diff contains only the five source files, two plan/log documents, and Sniff
skill file declared in the frontmatter. Pre-edit symbol impact was medium for
`run_isolated_software` (nine direct test callers, no production flows) and low
for each of the four parent-side Git test functions.

## Phase 5

Phase 5 migrated the remaining ordinary L1 CLI launches to the shared process
fixture, reduced the generic raw-spawn allowlist to zero, routed the retained
Unix PTY test through the builder's raw-command adapter, and removed the
unreachable bespoke PTY binary and dead library placeholder. Repository-aware
tests now build disposable repositories outside the checkout and opt into them
through the named ambient-context contract. No production parser, schema,
template, prompt, persistence format, configuration behavior, or CLI output
changed, so passive shipped-artifact corpus and read/write/read tests are not
applicable.

### Requirement-to-test mapping

| Requirement | Public observable proof |
|---|---|
| Owned fluent command migration | `owned_command_keeps_its_disposable_launch_directory_alive` invokes the real `sniff --help` binary after fluent builder use and asserts success, stdout help content, and empty stderr. The pre-fix form failed to compile because the temporary fixture could not outlive the returned command. |
| Full hostile-input contract | `inherited_git_sniff_and_rendering_inputs_are_scrubbed` supplies the exact probes `GIT_DIR`, `GIT_WORK_TREE`, relocated `HOME`/`XDG_CACHE_HOME`, a shadow-only `PATH`, `COLUMNS=44`, `FORCE_COLOR=1`, and checkout-ancestor-style `TMPDIR`; it asserts Git/rendering inputs are absent and downstream home, cache, temp, color, and PATH state comes from the fixture. |
| Repository-bearing CLI behavior | The 363-test `cli` target preserves exit status, stdout/text, JSON shapes, stderr, nested repository state, shallow-history behavior, version projection, blast-radius results, recent commits, packages, package areas, and negative behavior against test-built repositories. |
| Contract and satellite CLI behavior | The combined focused gate covers `install_plan` (9), `snapshots` (14), and `install_interview_cli` (1), including invalid manager/program errors, dry-run non-execution, cache creation/force replacement, help and structured-output snapshots, and the real dry-run interview path. |
| Raw PTY adapter | `os_subcommand_runs_in_pty` spawns the real binary through `command_std()` and asserts the OS heading appears before EOF under the retained Unix gate. |
| Zero generic exemptions and bespoke-gate removal | All four `spawn_site_guard` tests pass with an empty `SPAWN_ALLOWLIST`; the live scan reports no ordinary L1 raw sites, the detector mutation proves a violation is rejected, and stale entries are rejected. The guard excludes the two L2 files by tier. `install_interactive_pty.rs` was removed because its manufactured PTY adds no behavior beyond the reachable dry-run interview and no canonical recipe could set its bespoke gate; `foo.rs` was an empty placeholder. |
| Git fixture independence discovered by the full probe | Under the exact hostile `GIT_DIR`/`GIT_WORK_TREE`, `test_git_full_reports_conflicted_files` asserts the public full Git report contains conflicted files, and `structural_fixtures_match_canonical_git_when_available` compares the public conflict prediction against canonical Git across every structural fixture. Both failed before their parent-side Git launchers scrubbed inherited plumbing and passed afterward. |

### Validation

| Command | Selection | Result |
|---|---|---:|
| `cargo test -p sniff-cli --test cli --test cli_process_fixture --test spawn_site_guard --test tty --test install_plan --test snapshots --test install_interview_cli` | all Phase 5 CLI migration targets | pass: 401 tests, 0 failed |
| hostile `cargo test -p sniff --features remote --test integration test_git_full_reports_conflicted_files` | exact Git plumbing regression | pass: 1 |
| hostile `cargo test -p sniff --features remote --test merge_conflict_prediction structural_fixtures_match_canonical_git_when_available` | canonical Git oracle regression | pass: 1 |
| hostile `just test` | `GIT_DIR`, `GIT_WORK_TREE`, relocated home/cache, shadow PATH, width/color, and checkout-ancestor temp probes; `sniff/remote` + bare `sniff-cli` | pass: 2,609 passed, 24 skipped, 0 failed in 38.507 s runner time; 2.84 s warm build delta |
| `just lint` | final `sniff/remote` + bare `sniff-cli` state | pass: no diagnostics; 1 min 16 s cold check/build |

The first hostile full-area run exposed `test_git_full_reports_conflicted_files`
inheriting `GIT_DIR` and `GIT_WORK_TREE`; after the fixture was isolated, its
focused exact-input regression passed. The next full run reached
`structural_fixtures_match_canonical_git_when_available` and exposed the same
issue in its canonical Git oracle (`Non-fast-forward commit does not make sense
into an empty head`); its focused exact-input regression passed after the
oracle launcher was isolated. The final full probe then passed without changed
deterministic results. These were Phase 5 contamination findings, not accepted
pre-existing failures. The 24 final skips are the package area's declared
policy exclusions; there were no failures, retries, or timeouts in the final
run. The relocated home caused cold dependency setup on the first probe runs;
that build/setup cost is intentionally separate from the final runner time.

The disposable probe is retained at
`/Users/ken/.claudine/worktrees/sniff-phase5-probe.r6ZStX` for auditability; it
contains no user configuration. No retry, timeout, tier, or nextest override
changed in Phase 5. `git diff main -- .config/nextest.toml` remains the
unrelated pre-existing override cleanup and contains no Phase 5 change.

The required GitNexus compare-to-main `detect_changes` audit ran against the
shared linked worktree. It reported critical aggregate risk across 1,653 files,
10,031 changed symbols, and 126 affected symbols because this checkout already
contains extensive unrelated monorepo work. The Phase 5 path-scoped review is
limited to the twelve source paths and three documentation paths declared in
the plan frontmatter; it changes test infrastructure and fixtures only, with no
production execution flow. Pre-edit impact was LOW for the three merge-conflict
fixture helpers and the canonical Git oracle (one direct test caller each, no
production process).

## Phase 6

Phase 6 changed library tests only. No production behavior, parser, schema,
template, prompt, configuration, persistence contract, CLI channel, or
terminal rendering changed, so passive shipped-artifact corpus,
read/write/read, and CLI end-to-end additions are not applicable.

### Requirement-to-test mapping

| Requirement | Public observable proof |
|---|---|
| Remove incidental full discovery | `program_installable::test_installable_false_for_os_specific_programs` retains the exact `WindowsTerminal`/`TextMate` inputs and queries the public category detector without enumerating the host. The Windows-only SAPI sibling uses the same seam and remains pending its canonical CI leg. |
| Absent work is zero | `benchmark_workloads::formatting_workload_keeps_descendant_work_at_zero` asserts zero descendant walks, Git discoveries, and Git status walks while retaining formatting output setup. |
| Seeded execution and projection do not reacquire | `benchmark_workloads::seeded_git_execution_and_projection_do_not_rediscover_the_repository` separates acquisition/execution collectors and asserts `1` discovery during acquisition, then `0` discoveries, `0` opens, and exactly `2` requested status walks while also asserting dirty Git state and all four changed paths. |
| Collector propagation is trustworthy | Eight targeted tests cover scoped threads, generic pooled/Rayon workers, shared walker parity, manifest-walker reads, and the network thread hop; all passed. |
| Repeated fixture work is representative | `bench_fixtures::fixture_builders_are_idempotent_over_fresh_dirs` now compares two 10-package mixed repositories. The separate large fixture tests retain the 90-package, 21-commit, and cross-ecosystem dirty-state contract. |
| Weak equality assertions distinguish failures | `os::user::equality_is_by_variant_and_value` uses independently constructed lookup keys for same/different UID and SID values; `programs::types::test_executable_source_equality` compares independently deserialized wire variants and rejects cross-variant equality. |
| Snapshot identity/ordering/errors survive | Static review found no library golden-snapshot normalization. Existing remote captured-observation tests retain typed identity, selection, ordering, request-count, and error assertions. |

### Validation

| Command | Selection | Result |
|---|---|---:|
| `cargo test -p sniff --features remote --test program_installable --test bench_fixtures --test benchmark_workloads` | changed external library targets | pass: 10 tests; benchmark fixture target 0.71 s, workload target 0.22 s, installability target <0.01 s |
| focused Nextest expression | two strengthened equality tests plus scoped-thread, pool/Rayon, parallel-walker, manifest-walker, and network-hop counter proofs | pass: 8, 0 failed in 0.178 s |

The focused target added one test identity. The refreshed six-selection audit
capture records 2,636 distinct runner identities and the live source scan
records 2,677 test attributes. Reconciliation initially reported 18 expected
catalog drifts accumulated by the Phase 4/5 population changes; after adding
the two new CLI guard/fixture families, empty-family dispositions, and current
members/counts, the gate passed with 83 families, 41 configuration exclusions,
five known parse diagnostics, and no violations. An intermediate reconcile
also rejected two non-canonical disposition phrases; changing them to the
schema's `satisfactory` value resolved the error. These were audit-data
findings, not test failures.

| Command | Selection | Result |
|---|---|---:|
| test-audit `capture` | all six configured Sniff enumeration selections | pass: 2,636 distinct runner identities captured |
| test-audit `reconcile` | live sources, captures, families, and exact family members | pass: 83 families, 0 violations |
| `just test` | complete Sniff package area (`sniff/remote` + bare `sniff-cli`) | pass: 2,610 passed, 24 policy-skipped, 0 failed in 40.199 s |
| `just lint` | complete Sniff package area | pass: no diagnostics in 14.16 s |

There were no retries, timeouts, or pre-existing failures in the final run.
The 24 skips are the package area's declared policy exclusions. No doctest was
needed because no public API documentation or implementation behavior changed.
No retry, timeout, tier, or nextest override changed in Phase 6.

The required GitNexus compare-to-main `detect_changes` audit reported critical
aggregate risk across 1,656 files, 10,039 changed symbols, and 126 affected
symbols because the shared worktree contains extensive unrelated monorepo
changes. Phase 6's path-scoped change is limited to the five test-source files
and refreshed audit/plan documents declared in the frontmatter; it changes no
production execution flow. Pre-edit impact was LOW for each modified existing
test symbol, with no upstream callers or production processes.

## Phase 7

Phase 7 changes test ownership and synchronization only. It does not change a
production parser, schema, template, prompt, configuration, persistence, CLI
channel, or rendering contract, so shipped-artifact corpus and read/write/read
coverage are not applicable.

### Pre-implementation requirement-to-test mapping

| Requirement | Public observable proof |
|---|---|
| Deterministic remote tests never contact a live API | The remote-provider targets use loopback `MockServer` endpoints and assert provider results, exact request counts, bounded pagination, host consent, credential scope, and typed failures. The weak `from_shorthand_tries_github_when_token_set` test is removed: with its exact `test-owner/test-repo` input it called public provider APIs, accepted every transport error, and could not distinguish whether GitHub was tried. Existing local provider and `GitRemote` dispatch tests retain its meaningful coverage. |
| Readiness follows final terminal content | `level2_cicd_status_cells_render_styled_in_tmux` will poll until all visible rows/glyphs and green/red/dim SGR evidence are present; `level2_git_status_headers_and_links_render_styled_in_tmux` will poll until all sections, underline, hyperlink/fallback, and exact one-blank-row layout are present. Both retain the real shipped fixture binary invocation and their detailed final assertions. |
| Readiness failure remains bounded and diagnostic | The shared polling helper will have a fixed deadline and return the last captured frame; the existing public-output assertions then fail with the captured plain/raw terminal content rather than hanging. |
| Process termination, reaping, and semantic floors remain intact | Existing process and remote-refresh parent tests exercise the real timeout helper, drain output concurrently, assert timeout/termination state, and verify descendants disappear. The focused test gate and root leak sweep will prove runtime cleanup. |
| Owned servers and serialization remain scoped | Wiremock servers bind unique ephemeral loopback ports and stop on guard drop. Environment-mutating tests retain process-local serialization; the shared tmux pair retains its runner-visible shared harness/test group. The Windows L1 group remains unchanged pending the already-recorded two additional CI baselines. |

### Validation

| Command | Selection | Result |
|---|---|---:|
| `cargo test -p sniff --features remote --test remote_providers --test remote_observation --test focused_provider` | deterministic remote-provider cohort | pass: 135 tests, 0 failed; all network fixtures loopback-owned |
| focused Nextest process expression | 13 `process::tests` plus remote-refresh timeout | pass: 14 tests, including real drain, deadline, descendant, and reap paths |
| focused Nextest CLI expression | `tty::os_subcommand_runs_in_pty` | pass: 1 test under the new ten-second expect deadline |
| root `just test-leaks sniff` | scoped package-area `just test` plus detached-process sweep | pass: 2,609 tests, 24 policy skips, 0 failures in 41.013 s; no leaked processes |
| `just test-l2` | canonical shared-broker tmux route | pass: 2 tests in 0.678 s; no focus changes |
| test-audit `capture` + `reconcile` | six feature selections and all family/source mappings | pass: 2,633 distinct runner identities captured; 83 families, 41 declared exclusions, 0 violations |
| `just lint` | `sniff/remote` plus bare `sniff-cli` | pass: no diagnostics in 2.27 s |

The first proposed replacement for the live shorthand test attempted to prove
the no-credential path. Its focused run failed because provider constructors
still issue anonymous public probes and returned `ShorthandNotFound`; this was
the audit exposing the behavior, not a pre-existing suite failure. The weak
test was then removed, and the remote cohort, complete L1 suite, leak sweep,
and reconciled inventory all passed. No test retry, timeout override, tier,
production API, or `.config/nextest.toml` setting changed.

The required GitNexus compare-to-main change audit reports critical aggregate
risk across 1,658 files, 10,039 changed symbols, and 126 affected symbols
because this shared worktree contains extensive unrelated monorepo changes.
Phase 7 is path-scoped to the five test-source files and evidence documents in
the plan frontmatter. Pre-edit impact was LOW for both L2 test functions, the
PTY test, and the removed shorthand test: each had zero upstream callers and
no affected production process.

## Phase 8

Phase 8 changed evidence documents only. It added no product behavior, parser,
schema, template, prompt, configuration, persistence, CLI output, or test
behavior, so no new regression, passive-corpus, round-trip, or end-to-end test
was applicable. The observable verification map was the phase protocol itself:
full L1 and sanity claims use five alternating reports per revision; changed
PTY and tmux contracts use ten candidate executions; eliminated work uses the
Phase 1 counter pair plus Phase 6 sentinels; and the sanity budget uses measured
runner elapsed.

| Command/evidence | Selection | Result |
|---|---|---:|
| test-audit `measure run` | warm baseline/candidate, five alternating `just test` and `just sanity` rounds, four remaining candidate L1 loads, one tmux warmup plus nine tmux loads | pass: 38 commands, 0 nonzero exits |
| test-audit `measure report` | all retained logs, six changed cohorts, three changed timing contracts | gate pass; stable per-revision identities; no malformed/missing report inputs |
| candidate `just test` | ten complete Sniff-area L1 executions total | 2,609 passed and 24 policy skips per run; 0 failures, timeouts, leaks, or retries |
| candidate `just sanity` | one warmup plus five alternating samples | 1,820 passed and 23 policy skips per run; 0 failures, timeouts, leaks, or retries |
| candidate `just test-l2` | ten canonical runs with `BISCUIT_TEST_REQUIRED_BACKENDS=tmux` | 2 passed and 802 tier exclusions per run; 0 failures, timeouts, leaks, or retries; backend proof passed |
| counter validate/compare | `staged_filesystem_full_all_stages`, eight compatible signals | pass: eight readings valid and eight zero deltas |
| `just lint` | `sniff/remote` plus bare `sniff-cli` | pass in 4.22 s, no diagnostics |

Alternating medians kept build/setup, runner elapsed, and summed duration
separate. Full L1 baseline/candidate runner medians were 21.53/44.80 seconds
and summed medians were 324.99/684.81 seconds. Sanity runner medians were
7.94/17.03 seconds and summed medians were 104.80/254.82 seconds. The candidate
sanity spread of 16.73–19.17 seconds misses the 15-second budget on this host.
Host load rose from 43 to 134 during the alternating window, with Spotlight
indexing and a long-lived language-server process visible before the run. The
active candidate also contains 1,361 dirty monorepo paths. These numbers are
therefore local attribution only and establish no CI target.

The CLI integration cohort moved from 115.54 to 94.63 seconds summed (18.1%
lower but inside drift). The requested-work/fixture cohort moved from 23.45 to
4.30 seconds (81.7% lower, outside drift). Conversely, inherited-Git fixtures
moved from 29.89 to 51.17 seconds and the remote-provider target from 5.07 to
158.90 seconds. The remote file's only candidate change is deletion of the
weak live-network test; the remaining 70 tests are loopback-owned. The latter
regressions are retained as candidate-state/host confounders for matched Phase
9 CI evidence, not attributed to production behavior.

Each changed contract has exactly ten green candidate observations: the PTY
deadline ranged 0.304–0.446 seconds, the tmux CI/CD final-frame poll
0.317–0.378 seconds, and the tmux Git final-frame poll 0.340–0.374 seconds.
The unchanged production work-count case produced the same eight aggregates as
Phase 1, while the targeted Phase 6 counter tests remain the proof that seeded
execution and projection avoid rediscovery and that worker propagation is
complete.

Two report attempts failed before parsing evidence: one used zsh's reserved
`status` variable and one resolved a root-relative pnpm package path from the
package directory. The successful invocation used the established repository-
root form and both Markdown and JSON reports exit clean. Counter validation
likewise had one rejected wrong-base path invocation before the successful
validation/compare. An initially started release-profile counter run followed
the current example docs but did not match Phase 1's dev profile; it is retained
as a diagnostic and excluded. No test failure was retried or omitted.

GitNexus symbol impact and `detect_changes` are not applicable to this phase:
no function, method, class, or other Rust symbol changed, and no commit is being
created. Phase 8 is limited to the evidence and plan documents declared in the
frontmatter.

## Phase 9

Phase 9 completed the local pre-push tranche and the available tmux evidence.
The root consolidated recipe expanded to the exact two-package CI scope:
`sniff --features remote` and `sniff-cli --features test-fixtures`. Its first
run passed both test legs but found one Clippy `len_zero` diagnostic in the new
fixture test. GitNexus could not resolve the untracked test symbol and returned
UNKNOWN risk with zero indexed dependents. The assertion was rewritten without
changing its non-empty PATH contract, then the complete consolidated recipe
passed all five gates.

| Gate | Result |
|---|---|
| root `just ci-local sniff` | pass: CI-infra preflight, two package lints, 1,808 library tests, 801 CLI tests; 23/3 policy skips |
| area `just test` | pass: 2,609 tests, 24 policy skips |
| area `just lint` | pass, no diagnostics |
| area `just check` | pass |
| area `just doctest` | pass: 91 run, 22 ignored; CLI has zero doctests |
| required-tmux `just test-l2` | pass: 2 tests in 0.675 s; backend proof recorded two runs |

No real-resource behavior changed in Phase 9, so `just test-real` was not
applicable. The established reachability audit confirms that it enables
`network` for `sniff` and bare `sniff-cli`; it does not imply `remote` feature
coverage.

The CI tranche cannot begin from this session. The candidate remains
uncommitted at local HEAD `c2dee9217f3e6be14d7a6adfeb2c90cd2cd31966`, the
session may not stage, commit, or push, GitHub CLI is unauthenticated, and the
public GitHub API reports no current remote branch for
`fix/cli-slow-tests`. PR #69 previously used that branch name for unrelated
Claudine work and is already merged; it is not candidate evidence. The four
CI-dependent Phase 9 todos remain unchecked: candidate handoff/runs, explicit
Windows authority, matched per-environment comparison, and budget evaluation.
No missing evidence is labeled passing.

The required GitNexus compare-to-main audit reports CRITICAL aggregate risk
across 1,658 changed files, 10,039 changed symbols, and 126 affected symbols.
That is the shared dirty-worktree result, not the risk of Phase 9's one-line
test assertion correction. The pre-edit lookup for that new untracked test
symbol returned UNKNOWN with zero indexed dependents. A path-scoped
`git diff --check` passes for all five Phase 9 files. The repository-wide form
still reports pre-existing trailing whitespace in `prompts/commit.md`, which
is outside this package area and was not modified.

## Phase 10

Phase 10 changed closure documents only. It introduced no product or test
behavior, so its requirement-to-test mapping is the final evidence ledger:
existing targeted tests prove each changed behavior, reconciliation proves
identity coverage, the leak sweep proves process cleanup, and the canonical
area gates prove the unchanged source state. No parser, schema, shipped
artifact, configuration, persistence, or terminal-output contract changed, so
no passive corpus, round-trip, or new end-to-end test was applicable.

| Command | Selection | Result |
|---|---|---:|
| test-audit `config validate` | Sniff audit configuration | pass: two packages, six selections, four environments |
| test-audit `reconcile --markdown` | current captures, sources, 83 families | pass: 2,635 runner identities, 41 declared platform exclusions, zero violations |
| `/usr/bin/time -p just sanity` | `sniff/remote` + `sniff-cli/test-fixtures`, lib + bins, `!slow` | pass: 1,419 library and 401 CLI tests; 11.72 s wall clock against the 15-second budget |
| root `just test-leaks sniff` | Sniff-area L1 | pass: 2,609 tests, 24 policy skips, 0 failures in 22.470 s; no leaked processes |
| root `just check-tier-coverage sniff` | Sniff stubbed L3/browser tiers | initial environment failure under macOS Bash 3.2; pass under installed Bash 5.3 with zero stranded tests |

The default tier-audit invocation failed before listing tests because the
shared recipe uses `BASHPID`, which macOS `/bin/bash` 3.2 does not define under
`set -u`. After diagnosis, the second invocation prepended the already-
installed `/opt/homebrew/bin/bash` 5.3 to `PATH`; the audit then passed. No
shared tooling was changed in this Sniff-scoped phase.

The final ledger credits Phase 9's still-applicable `just lint`, `just check`,
`just doctest`, and required-tmux `just test-l2` results. `just test` was
covered again by the Phase 10 leak-sweep wrapper. The final nextest diff adds no
Sniff override, retry, tier change, disabled assertion, or timeout increase;
the surviving `sniff-windows-l1` group remains justified in `inventory.md`.

`results.md` now carries the complete measurement, coverage, work-count,
budget, failure/skip, local-gate, pending-CI, and nine-criterion acceptance
review. There are no permitted deferrals or generic migration exemptions.
AC6 and AC8 remain pending because the exact candidate is uncommitted by
explicit instruction and therefore has no candidate CI SHA, native-Windows
execution, or three-run matched environment series. Those are required pending
evidence, not deferrals; the Phase 9 plan and inventory sections own the
handoff. The fix is implemented and verified locally but is not verified on CI
and must not be archived yet.

### Phase 10 continuation — candidate committed, gates re-run

The candidate has since been committed by the separate commit step. Its Sniff
source content is `66892d319` (last commit touching `sniff/lib` or `sniff/cli`)
with the fix documents at `3b112c4d5`; the branch head carrying that tree is
`a05e3b747`. `git status --porcelain` is empty, so the working tree and the
committed candidate are identical.

The branch is **53 commits ahead of `origin/fix/cli-slow-tests`** and has not
been pushed. `gh run list --branch fix/cli-slow-tests` shows the newest CI run
(`34159725015`, `ci`, success) against head SHA
`a9e88c069…`, which is the current origin head; `git merge-base --is-ancestor
66892d319 origin/fix/cli-slow-tests` returns false. **No CI run covers the Sniff
candidate.** The stale reason recorded above ("uncommitted") is superseded: the
candidate now exists as a commit, but it is unpushed, so the AC6/AC8 CI evidence
is still absent for the same practical effect. Pushing is the human-gated
Phase 9 handoff and is out of this phase's scope.

Because unrelated Darkmatter and Claudine commits landed after the Sniff
candidate, every local gate was re-run at the current tree rather than credited
from Phase 9/10's earlier ledger. No Sniff source file changed between the two
ledgers, and every result reproduced.

| Command | Selection | Result |
|---|---|---:|
| test-audit `config validate` | Sniff audit configuration | pass: 2 packages, 6 selections, 4 environments |
| test-audit `reconcile --markdown` | current captures, sources, 83 families | pass: 2,635 runner identities, 2,676 source attributes, 41 declared platform exclusions, 5 known parse diagnostics, `GATE EXIT=0` |
| `/usr/bin/time -p just sanity` (cold) | `sniff/remote` + `sniff-cli/test-fixtures`, lib + bins, `!slow` | pass: 1,419 + 401 tests; 142.16 s wall including a full rebuild of the changed dependency graph |
| `/usr/bin/time -p just sanity` (warm) | same | pass: 1,419 (23 skipped) + 401 tests; runner 9.418 s + 0.506 s; **12.13 s wall clock, within the 15-second budget** |
| `/usr/bin/time -p just lint` | `sniff/remote` + `sniff-cli` | pass: no diagnostics, 50.70 s |
| `/usr/bin/time -p just check` | `sniff/remote` + `sniff-cli` | pass, 42.17 s |
| `just doctest` | `sniff/remote`; `sniff-cli` | pass: 91 passed, 0 failed, 22 ignored; CLI has zero doctests |
| `/usr/bin/time -p just test` | Sniff-area L1 | pass: 2,609 run, 2,609 passed, 24 skipped, 0 failed; runner 23.855 s, 86.34 s wall |
| `/usr/bin/time -p just test-l2` | `sniff-cli --features test-fixtures`, tmux | pass: 2 tests in 0.655 s (802 skipped); 14.35 s wall |
| root `just test-leaks sniff` | Sniff-area L1 | pass: 2,609 tests, 24 skips, 0 failures in 23.477 s; `leak-sweep: no leaked processes detected` |
| root `just check-tier-coverage sniff` | Sniff stubbed L3/browser tiers | pass under Bash 5.3: 1 listed, 0 stranded |
| `git diff main -- .config/nextest.toml` | override delta | 16 insertions / 41 deletions; grepping the diff for `sniff` or `detect_completes` returns nothing — this fix changed no Sniff override |

`check-tier-coverage` was invoked directly under `/opt/homebrew/bin/bash` 5.3.
The macOS Bash 3.2 `BASHPID` limitation recorded in the previous Phase 10 entry
is unchanged and still owned by the shared tooling, not by this fix.

No new failure, retry, timeout, slow mark, or leak appeared. AC6 and AC8 stay
**pending on CI** with their reason updated from "uncommitted" to "committed but
unpushed; no CI run covers the candidate tree".

## Implementation of Review Findings #1

> **started at:** 2026-09-09T13:37:52-07:00

- this implementation is attempting to implement _all_ of the review findings
  found in 'sniff/fixes/2026-09-07-faster-sniff-tests/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- **blocked before any finding could be started:** the named review file does
  not exist and no implementation work was performed
        - `sniff/fixes/2026-09-07-faster-sniff-tests/review-1.md` is absent
          from the working tree; the fix directory contains `spec.md`,
          `plan.md`, `inventory.md`, `results.md`, `log.md`, the audit
          configuration, and the baseline/enumeration/measurement evidence
          trees, but no review artifact
        - `git log --all -- 'sniff/fixes/2026-09-07-faster-sniff-tests/review*'`
          returns nothing, so the file was never committed and then deleted on
          any reachable ref — the delta review that produces it has not been run
          for this fix
        - the only uncommitted change in this fix directory is `spec.md`
          flipping `implemented: false` → `implemented: true` (plus one trailing
          blank line removed), which is the trigger for the review-to-implement
          cycle but is not itself a source of findings
- no finding was fabricated, no source file was modified, and neither `just
  test` nor `just lint` was run, because there was no change to verify

### Blocked

The implementation of review cycle 1 did not run. Iterating over review
findings requires the review artifact for this fix, and
`sniff/fixes/2026-09-07-faster-sniff-tests/review-1.md` has never existed in
this repository. Zero findings were evaluated, zero were fixed, and zero were
deferred — this is a missing-input blocker, not a deferral, so
`deferred_perf_measurement` is not applicable and no deferred-measurement
document was created.

To unblock: run the delta review for this fix so that
`review-1.md` is produced from the difference between
`sniff/fixes/2026-09-07-faster-sniff-tests/spec.md` and the implemented Sniff
and Sniff CLI test sources, then re-run this implementation cycle. No
metadata was written to `review-1.md` (it does not exist), and the log
frontmatter `implementation_1` was intentionally left unset because no
implementation occurred.

Note for the reviewer that runs next: the fix's own closure evidence already
records two acceptance criteria as pending on CI — AC6 and AC8 — because the
candidate tree is committed but unpushed and no CI run covers it. That is
pre-existing state carried in from Phase 10, not a finding from this cycle.

## Implementation of Review Findings #1

> **started at:** 2026-09-09T15:04:39-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Users/ken/.claudine/worktrees/rusty-biscuit/fix-cli-slow-tests/sniff/fixes/2026-09-07-faster-sniff-tests/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review artifact that the previous cycle reported as missing now exists (created 2026-09-09T13:38:56-07:00, amended 13:50 with Ken's `DECISION` block on the CI finding), so this cycle proceeds
- the review carries three findings
        - **High** — no candidate CI or native-Windows verification exists; Ken's inline `DECISION` reduces this to local cross-OS execution on the three declared build hosts rather than a production-readiness gate
        - **High** — the recorded before/after comparison does not prove the faster-tests outcome
        - **Medium** — the fluent CWD escape does not enforce fixture ownership or justification
- host load at start was `load averages: 21.69 33.50 28.08` on the 10-core arm64 macOS host, which is already outside a legitimate measurement window for the second finding; this will be re-checked before that finding is attempted
- ordering decision: the Medium code finding is implemented first, because the two High findings are execution/measurement evidence that must be gathered against the final candidate tree
- starting the work on 'Medium — fluent CWD escape ownership and justification' at 15:07:41
- discovery: `assert_disposable_context` (`sniff/cli/tests/common/mod.rs`) backed both fluent forms — `OwnedSniffCommand::ambient_context` and `impl DisposableAmbientContext for assert_cmd::Command` — and rejected only paths inside the checkout, so `/usr/bin`, a developer repository, or any `$HOME` subdirectory was accepted
        - all 42 live `ambient_context` call sites are in `cli.rs` and every one of them derives its path from `tempfile::tempdir()` (verified by reading each producer: `create_test_repo`, `create_cli_monorepo`, `create_test_repo_with_worktree`, and the inline `tempfile::tempdir()` bindings), so tightening to the system temporary root cost no call-site churn
        - the four live PATH escapes are `cli.rs:15`, `cli_process_fixture.rs:164`, `cli_process_fixture.rs:168`, `install_interview_cli.rs:14`; only two of the four had a comment on a line the guard could attach to
- change 1 — ownership by construction: `assert_disposable_context` now also requires the canonicalized directory to sit under a canonicalized `std::env::temp_dir()`, with a second, distinctly worded panic naming the fixture-owns-its-inputs contract; both sides are canonicalized so macOS `/var/folders/...` matches `/private/var/folders/...`
- change 2 — negative tests in `cli_process_fixture.rs`: four `#[should_panic(expected = ...)]` tests, one per (owned form, fluent trait form) x (inside the checkout, `common::minimal_system_path()` system directory), plus a positive `ambient_context_accepts_a_disposable_directory_the_test_built`
- change 3 — `spawn_site_guard.rs` grew a PATH-escape detector reusing `source_scan::{sanitize, is_ident, line_at}`; it finds `.fake_only_path(` / `.host_path(` method calls only (receiver `.` required, trailing-ident and bang checks as in the spawn detector) and requires an attached `//` reason: the site's own trailing comment, else the contiguous whole-line comment block ending at the nearest preceding non-blank line
        - a `//` sequence inside a string literal is not a reason: the sanitized line must be blank from the `//` to end of line, which `let banner = "// git is the tool";` fails on its trailing `;`
        - `scan` was refactored to share a `scanned_sources` file walk with the new `scan_path_escapes`
- change 4 — justification comments written at all four escape sites; `cli_process_fixture.rs:167`'s claim that "`git` is the real tool this escape intentionally exposes" was drifted (the test only inspects PATH entries, it never runs git) and was replaced with the escape-ordering reason the assertions actually prove
- non-vacuity, proved by mutation rather than asserted
        - deleting the new comment at `cli.rs` made `l1_path_escapes_carry_a_call_site_reason` fail with `cli.rs:15 fake_only_path`; restored
        - short-circuiting the temporary-root assertion made both `*_rejects_a_non_disposable_directory` tests fail; restored
        - unit tests mirror the existing pair: `escape_detector_finds_both_path_escapes_and_ignores_prose_and_strings`, `escape_justification_accepts_adjacent_comments_and_rejects_string_lookalikes`, and `restored_escape_detector_rejects_an_unjustified_site_that_a_neutered_detector_misses` (a detector with `host_path` switched off misses the site the restored one catches)
        - the live scan also asserts it found a non-zero number of sites, so a detector that stops observing the suite fails instead of passing empty
- drift: `.claude/skills/sniff/SKILL.md` said all three escapes need a call-site reason; updated to record that `ambient_context` is now structural and only the two PATH escapes carry comments
- `just --justfile sniff/justfile test`: 2618 tests run, 2618 passed (13 slow), 24 skipped, summary 63.281s, 2m20s wall including the build
- `just --justfile sniff/justfile lint`: clean, 1m07s wall (warm re-run; the first run was 1m36s)
- also ran `cargo clippy -p sniff-cli --tests -- -D warnings` (clean) because the area `lint` recipe does not pass `--all-targets` and so does not lint test binaries; `cargo fmt -p sniff-cli --check` is clean and no formatting pass was run
- work completed for 'Medium — fluent CWD escape ownership and justification' at 15:27:41
- starting the work on 'High — cross-OS execution on the declared build hosts' at 15:29:33
        - Ken's `DECISION` in the review supersedes the CI clause: candidate CI is no longer the production-readiness gate, and the finding is discharged by running the Sniff suites locally on `build-linux` (Linux), `build-win-native` (native Windows), and `build-win` (WSL2)
        - all three hosts answered `ssh -o BatchMode=yes`; each already carries a standing clone (`~/ci-verification/rusty-biscuit` on the two Linux-side hosts, `W:\ci-verification\rusty-biscuit` on native Windows), and all three were left dirty by earlier runs
        - the repo already has the right vehicle: root `just cross-check <package> [--host linux|windows|all]` resets/cleans the standing clone, detaches it at the newest fetchable base, applies the local tree as one patch, and runs the package suite there; it covers Linux and native Windows but has no WSL host, so WSL is driven by hand with the identical reset/clean/detach/apply/run sequence
        - `origin/fix/cli-slow-tests` is not present in this worktree, so the script falls back to `origin/main`; the shipped patch is therefore the whole branch delta, 1,473 files and ~64 MB
        - **contamination discovered and worked around:** the standing clones are shared, and another concurrent session reset all three of them to `2b5b50eb4` (a commit that is not an ancestor of this branch) in the middle of this work
                - the first `just cross-check sniff-cli --host all` result (804 passed / 3 failed) and the first native-Windows re-run (2,573 passed / 4 failed, 3 of them `0xc0000409` stack-buffer-overrun aborts in the `sniff` native detectors) were both taken against a tree that was **not** the candidate, so neither is admissible evidence
                - a second Linux attempt on the shared clone died with `output file ... libquote-...rmeta is not writeable`, i.e. another session's cargo owning the shared `target/`
                - every subsequent run therefore does reset → detach → `git apply` → run inside **one** ssh session and prints a `PRE-RUN-MARKER`/`POST-RUN-MARKER` pair carrying the HEAD sha, the dirty-file count, and a candidate-only marker (`temp_dir` in `sniff/cli/tests/common/mod.rs`, which is absent before this cycle's change); a result is only admissible when both markers agree and show the candidate
                - Linux and WSL additionally run in a private `git worktree` with a private `CARGO_TARGET_DIR`, which their free space allows (161 GB and 125 GB); native Windows keeps the shared clone and its warm target because `W:` has only 21 GB free and the host's storage policy forbids a scratch target there
        - **Linux (`build-linux`), verified candidate:** `PRE/POST-RUN-MARKER: 0a2b4cf8b dirty=895 guard=1` on both sides, private worktree `~/rb-sniff-review1` with private `CARGO_TARGET_DIR` — `cargo nextest run -p sniff-cli -p sniff --no-fail-fast`: **2,605 run, 2,605 passed, 15 skipped, 0 failed** in 7.451 s
        - **native Windows (`build-win-native`), verified candidate:** `PRE/POST-RUN-MARKER: 0a2b4cf8b dirty=895 guard=3` (the Windows marker count is 3 because `Select-String` is case-insensitive and also matches `TEMP_DIR_VARIABLE`) — **2,597 run, 2,593 passed, 8 skipped, 4 failed** in 66.960 s
                - the four new fluent-escape negative tests from the Medium finding all **pass** on native Windows, including the `\\?\W:\...` checkout rejection and the `C:\WINDOWS\System32` non-disposable rejection, so the Windows command adapter, `SystemRoot`/`COMSPEC`/`PATHEXT` restoration, and `cmd.exe /D /C set` recorder are now exercised on the platform the review said had never run them
                - two failures are **real, pre-existing, native-Windows-only defects in this fix's own process fixture**, both `called Option::unwrap() on a None value` on `environment.get("PATH")`: `inherited_git_sniff_and_rendering_inputs_are_scrubbed` (`cli_process_fixture.rs:136`) and `path_modes_are_bounded_and_keep_fixture_stubs_first` (`cli_process_fixture.rs:153`)
                        - root cause: `parse_environment` keys the recorded environment case-sensitively, but Windows environment variables are case-insensitive and `cmd.exe /D /C set` reports the canonical `Path=`, not `PATH=`
                        - the same defect makes `assert_and_raw_surfaces_produce_the_same_effective_environment`'s `PATH` equality assertion **vacuous on Windows**: it compares `None` to `None` and passes
                        - this is exactly the class of gap the review predicted ("its Windows command adapter, case-insensitive environment behavior, `cmd.exe /D /C set` recorder ... cannot be exercised by macOS or WSL"), so the finding paid for itself
                - two failures are `0xc0000409` stack-buffer-overrun **aborts in the `sniff` library's native detectors** (`tests::test_os_present_by_default`, `integration::test_detect_completes_in_reasonable_time`); a control run is being taken at the unpatched base to establish whether they pre-date this fix
- fixed the native-Windows `PATH` lookup defect in `sniff/cli/tests/cli_process_fixture.rs` at 15:48:31
        - `parse_environment` now upper-cases the recorded variable name on Windows only, so the recorded map models the platform it reads from: Windows names are case-insensitive and `cmd.exe /D /C set` reports each under its canonical casing (`Path=`, never the `PATH=` the builder exported), while Unix names stay case-sensitive because the Unix environment itself is
        - normalizing at the point of *recording* rather than at each lookup keeps all five existing upper-case `get`/`contains_key` call sites unchanged, which is the surgical shape: no test body, no assertion, and no isolation-policy code moved
        - `to_ascii_uppercase` rather than a Unicode upper-casing: the names under assertion are all ASCII, and two Windows variables differing only in ASCII case cannot coexist, so the normalization cannot collapse distinct entries
        - the `=C:=C:\...` pseudo-variables `cmd.exe` emits are untouched by this: `split_once('=')` still yields an empty key for them, exactly as before
        - the vacuous assertion is repaired: `assert_and_raw_surfaces_produce_the_same_effective_environment` previously compared `assert_environment.get("PATH")` with `raw_environment.get("PATH")`, and on Windows both sides were `None`, so the assertion held while proving nothing about the two command surfaces agreeing on `PATH` — the very thing it exists to prove. Both sides are now unwrapped through `expect`, so a missing `PATH` on either surface fails the test instead of satisfying it
        - the two hard failures (`cli_process_fixture.rs:136` and `:153`) need no edit of their own: their `environment.get("PATH").unwrap()` calls were already non-optional and now resolve, because the key they look up is the key the map records
- verification at 15:48:31
        - `just --justfile sniff/justfile test` — **2,618 run, 2,618 passed, 24 skipped, 0 failed** in 26.570 s (macOS host); the `cli_process_fixture` binary alone re-run as `just test --test cli_process_fixture` — 13 passed, 1 skipped
        - `just --justfile sniff/justfile lint` — clean
        - **Windows compile evidence, not a behavioral run:** added a `check-windows` recipe to `sniff/justfile`, mirroring claudine's (mingw `x86_64-pc-windows-gnu`, `RUSTC_WRAPPER=""`, `CFLAGS/CXXFLAGS_x86_64_pc_windows_gnu=-Wa,-mbig-obj`, scratch `CARGO_TARGET_DIR=target/windows-check` so the main target dir is undisturbed). `just --justfile sniff/justfile check-windows` finishes clean in 1 m 04 s; the only warnings are pre-existing ones in `sniff/lib` (`executable_index.rs` unused bindings, `git_parity.rs` dead `stage_raw_path`), none in the changed file. The msvc target is deliberately not used — it dies in `aws-lc-sys` for want of the Windows SDK
        - a cross-compile proves the `cfg(windows)` arm compiles; it does not execute it. The behavioral proof still owed is a native-Windows run of `sniff-cli::cli_process_fixture` on `build-win-native`, where the expected delta is the two `unwrap` panics turning into passes and the third test's `PATH` comparison becoming real rather than `None == None`
        - the two native-Windows fixture failures were repaired (see the `parse_environment` bullets above), the patch was regenerated, and native Windows was re-run as a **matched control-then-candidate pair in one ssh session**, same host, same selection `-p sniff-cli -p sniff --no-fail-fast`, back to back
                - control, unpatched base `0a2b4cf8b` with `dirty=0`: **2,577 run, 2,574 passed, 3 failed** in 80.075 s — `sniff tests::test_skip_os_returns_none` and `sniff tests::test_detect_returns_result` aborted with `0xc0000409`, and `sniff-cli::cli test_no_subcommand_with_json_outputs_json` failed
                - candidate, base + patch with `dirty=896 v2guard=1`: **2,597 run, 2,597 passed, 8 skipped, 0 failed** in 68.349 s
                - so the `0xc0000409` aborts are **pre-existing at the base and absent from the candidate**; the aborting test varies between runs (`test_os_present_by_default`, `test_non_loopback_has_mac_address`, `test_skip_os_returns_none`, `test_detect_returns_result` in different combinations), which marks them as a load-sensitive native-detector flake on this host rather than anything this fix introduced. They are recorded here as an unrelated pre-existing native-Windows defect, not a deferral of this fix
                - an earlier single-package control (`-p sniff` alone, 1,517 passed) did **not** reproduce the aborts, which is why the matched two-package selection was re-run before drawing any conclusion
        - a Linux re-run against the updated patch first died with `output file ... .rmeta is not writeable`; the artifacts are `-r--r--r--` with a link count of 2, i.e. read-only hardlinks from the shared build cache that an incremental rebuild cannot overwrite. Re-running into a fresh `CARGO_TARGET_DIR` is the workaround; this is a build-cache interaction on the host, not a Sniff defect
        - **all three declared hosts are green on the verified candidate**, every run marker-matched before and after
                | Host | Environment | Selection | Result | Elapsed |
                |---|---|---|---:|---:|
                | `build-linux` | Linux x86_64, Proxmox guest | `-p sniff-cli -p sniff --no-fail-fast` | 2,605 run, **2,605 passed**, 15 skipped | 7.430 s |
                | `build-win` | WSL2 (`6.18.33.2-microsoft-standard-WSL2`) | `-p sniff-cli -p sniff --no-fail-fast` | 2,605 run, **2,605 passed**, 15 skipped | 13.443 s |
                | `build-win-native` | native Windows, PowerShell | `-p sniff-cli -p sniff --no-fail-fast` | 2,597 run, **2,597 passed**, 8 skipped | 68.349 s |
                | this host | native arm64 macOS 27.0 | `just --justfile sniff/justfile test` | 2,618 run, **2,618 passed**, 24 skipped | 26.570 s |
        - remote scratch was cleaned up afterwards: both private worktrees removed and pruned, `~/rb-sniff-review1-target` (1.4 GB), `~/rb-sniff-r1t-v2` (1.1 GB) and `~/rb-sniff-r1t` (5.9 GB) deleted, patch files removed from all three hosts
        - **operator note for Ken:** `W:` on `build-win-native` is down to **10.2 GB free** after this work (it was 21 GB at the start). The consumption is inside the shared standing clone's own warm `target/`, which other sessions depend on, so it was deliberately left in place rather than swept by this cycle
- work completed for 'High — cross-OS execution on the declared build hosts' at 16:01:44
- starting the work on 'High — the recorded comparison does not prove the faster-tests outcome' at 16:05:12
        - the finding was **not** deferred on load, because the load average turned out to be misleading: `top -l 4 -s 5` reported a sustained **80–84 % idle CPU** across four samples while the 1-minute load average sat at 10–11 on 16 cores, i.e. the queue is I/O-bound (`identityservices` at ~90 % of one core), not CPU-bound. There is real headroom for a measurement
        - the two trees the review asked for both exist and can be pinned clean
                - baseline: the preserved detached worktree `target/sniff-phase1-baseline-c2dee9217` at `c2dee9217`, tracked-clean, with its warm build directory still on disk (6.9 GB `target/` plus 3.7 GB `target-sniff-phase1/`)
                - candidate: a new detached worktree `/private/tmp/rb-sniff-cand-review1` at `fe83e7481` with only this cycle's 718-line Sniff diff applied, so it is pinned and carries none of the unrelated monorepo dirt the review objected to in `provenance.json:24`
                - `c2dee9217` is an ancestor of `fe83e7481`, 28 commits back, and the Sniff delta between them is 21 files / 2,029 insertions / 962 deletions
- the alternating local measurement the review asked for was run in full; artifacts in `measurement/review1-alternating/` (`summary.md`, `provenance.json`, `runs.jsonl`, 20 per-run logs, 3 warm-up logs, the `run.sh` driver)
        - **protocol actually run.** invocation form `just --justfile <TREE>/sniff/justfile --working-directory <TREE>/sniff <cohort>`, verified working before measuring and used unchanged — the sniff justfile's `../just/*.just` imports resolve relative to the justfile, and the nested `just _test_local_all` / `just _sanity_all` calls re-resolve from the working directory, so every invocation stays inside its own tree with no `cd`
        - both trees were warmed twice per cohort with all warm-up numbers discarded. the candidate had no `target/` at all (cold) and the baseline's pre-existing 6.9 GB `target/` also turned out **not** to be warm for this recipe's feature unification — both trees recompiled 787 crates on warm-up pass 1, fast only because `kache 0.19.0` is the `rustc-wrapper`. pass 2 confirmed both fully warm (`Finished test profile … in 0.26–0.37 s`, zero `Compiling`)
        - 5 alternating rounds in the order baseline `test` → candidate `test` → baseline `sanity` → candidate `sanity`; 20 measured runs, **zero `Compiling` lines and exit 0 with every test passing in all 20**, so no run had to be discarded. each run recorded `/usr/bin/time -p` wall clock, the verbatim nextest `Summary` line(s), `uptime` load before and after, and a sustained `top -l 2 -s 3 -n 0` idle sample before
        - **sample valid**: baseline stayed at `c2dee9217` with 1 untracked entry and the candidate at `fe83e7481` with 9 dirty files, identical before the first measured run and after the last
        - **paired wall clock, `test` cohort (s)**: r1 20.37/20.24 (0.994) · r2 20.22/21.69 (1.073) · r3 24.41/30.99 (1.270) · r4 36.63/30.42 (0.830) · r5 23.66/23.36 (0.987) — baseline median **23.66** (range 20.22–36.63), candidate median **23.36** (range 20.24–30.99), median ratio **0.987 (−1.3 %)**
        - **paired wall clock, `sanity` cohort (s)**: r1 10.14/11.38 (1.122) · r2 10.24/12.01 (1.173) · r3 15.29/17.77 (1.162) · r4 11.10/11.26 (1.014) · r5 13.08/15.27 (1.167) — baseline median **11.10** (range 10.14–15.29), candidate median **12.01** (range 11.26–17.77), median ratio **1.082 (+8.2 %)**, median paired ratio **1.162**
        - nextest runner-elapsed medians agree: `test` 22.58 → 22.33 (−1.1 %), `sanity` 8.36 → 9.35 (+11.8 %, median paired ratio 1.189)
        - **drift bracket.** using the spread of the baseline runs as the noise floor: `test` wall 16.41 s = **69.4 %** of the baseline median, `test` runner 72.6 %, `sanity` wall 5.15 s = **46.4 %**, `sanity` runner 57.3 %. **every observed difference is inside its bracket**, so no percentage above may be reported as a speedup or a slowdown magnitude
        - the bracket does not absorb the *direction* of the sanity result: the candidate's paired sanity ratio is above 1 in **5 of 5 rounds** on both wall clock and runner elapsed (one-sided sign test p ≈ 0.03). directionally real, magnitude unresolved
        - **load anomalies.** idle stayed in the 61–88 % band for 19 of 20 runs. two deviate — round 4 baseline `test` (61.4 % idle, load 100.69 after; 36.63 s, the slowest run in the sample and the sole reason the baseline range reaches 36.63) and round 5 candidate `sanity` (**10.4 % idle**, genuine external saturation). excluding them pairwise changes nothing: `test` becomes 22.02/22.53 = 1.023 (+2.3 %, sign flips, still deep inside the bracket — itself evidence the `test` number is noise), `sanity` becomes 10.67/11.70 = 1.096, and dropping rounds 3 and 5 together gives 10.24/11.38 = 1.111
        - **15-second `sanity` budget, both trees.** on the wall clock a developer actually waits for, baseline is median 11.10 s / max 15.29 s and breaches 15 s in **1 of 5** rounds; candidate is median 12.01 s / max 17.77 s and breaches in **2 of 5**. on nextest runner elapsed neither breaches (baseline median 8.36 / max 12.58; candidate median 9.35 / max 14.96) but the candidate's round 3 left **45 ms** of headroom. verdict: the budget is met at the median in both trees and is **not robustly met under load in either**, and the candidate has strictly less headroom on every measure. the ~2.6–5 s gap between the two measures is fixed recipe cost — `_storage_preflight` plus two separate `cargo metadata` + `cargo nextest` invocations, one per package
        - **test counts differ and are reported separately.** `test` cohort 2,599 (23 skipped) baseline vs 2,618 (24 skipped) candidate, **+19** — the candidate added coverage. per-test mean at the median runner elapsed is 8.688 ms → 8.531 ms (−1.8 %), recorded but **not** the headline: cohort elapsed is a parallel wall clock across 28 binaries, so dividing by a test count does not yield per-test cost, and the figure derives from a −1.1 % cohort difference 65× smaller than the drift bracket. the honest headline is the raw cohort wall clock, 23.66 s → 23.36 s, within drift. the `sanity` cohort's counts are **identical** (1,419 + 401 = 1,820 in both trees), so its per-test ratio equals its cohort ratio
        - **the sanity regression is not the fix's doing, and the control proves it.** `sanity` is `--lib --bins`, which never builds `sniff/cli/tests/*` — where the entire 521-line working diff lives. across `c2dee9217..fe83e7481` the only `sniff/lib/src` or `sniff/cli/src` hunks are `os/user.rs` and `programs/types.rs`, both **entirely inside `#[cfg(test)] mod tests`** (assertion rewrites costing a two-entry `HashMap` and two `serde_json` parses). so the sanity cohort is a near-perfect control: identical test population, effectively identical code, and it still moved +9–19 %
        - per-test attribution over the three quietest rounds (1,820 tests joined by name) shows the delta is concentrated, not diffuse — 956 tests slower, 790 faster, top 20 accounting for 11.69 s of the 20.30 s total. the top five are all `filesystem::docs::tests::integration::*` at +1.12 to +1.28 s each, followed by ten `filesystem::git::worktree::tests::list_worktrees_*`/`inside_*` and then `filesystem::blast_radius::tests::…::*_in_real_repo`. **every one of them walks the real surrounding repository**
        - and the candidate's repository is bigger because this fix cycle committed its own measurement artifacts into it: tracked files 10,963 → 11,358 (**+395**), tracked `.md` 4,140 → 4,174 (**+34**), `sniff/fixes/` 4.6 M → 14 M (**+9.4 M**). the five `docs` tests enumerate and content-hash every Markdown document in the repo, so 34 more documents and 9.4 MB more to traverse is a direct and sufficient explanation for the largest deltas. the regression is a **self-inflicted, incidental cost of the branch's committed artifacts**, not a property of the test-speed work — but it is still a cost a developer running `just sanity` on this branch pays
        - two consequences worth carrying forward: (a) any future between-tree comparison in this repo must place both trees in structurally comparable locations **and** account for tracked-content volume, because Sniff's own suite measures the repository it sits in — the `sniff` instance of the known "capture tests walk the monorepo" hazard; (b) the systematic offset the control exposes means the `test` cohort's −1.3 % was measured against a candidate carrying a handicap on the repo-walking tests it shares, which *could* mean its real change is more favourable — **this is deliberately not claimed**, since the cohorts have different populations and I/O profiles and subtracting the offset would manufacture a win the data does not support
        - **conclusion: the candidate is not measurably faster on either cohort.** the full L1 `test` cohort shows no demonstrated change in either direction (−1.3 %, inside a 69 % bracket, paired ratios split 3 up / 2 down), and the `sanity` cohort is consistently *slower* (5 of 5 rounds) for reasons traceable to committed artifacts rather than to the test changes. **review-1's finding stands: this measurement does not demonstrate a faster-tests outcome.** what the candidate does show is that it runs 19 more tests in the same time without regressing the full suite — which is not a speedup
        - the CI half of the finding (matched CI samples, per-family budget ratification) was **not** attempted here and remains blocked on the unpushed, human-gated branch
- the 6.6 MB of raw per-run logs were archived into `raw-logs.tar.gz` (1.0 MB) rather than committed loose, because this measurement's own control finding is that committed artifact volume is what slowed the `sanity` cohort; `summary.md`, `provenance.json`, `runs.jsonl`, `run.sh`, and `driver.log` stay plain
- closure documents updated: [`results.md`](results.md) gained a `## Review-1 implementation outcomes` section, AC2 and AC6 moved to **Verified**, AC8 moved to **Partially verified** with the negative timing result stated plainly, the native-Windows pending row is struck through as resolved, and the completion-claims table gained an explicit `Faster tests demonstrated | No` row; [`deferred-performance-measurement.md`](deferred-performance-measurement.md) records the one outstanding clause
- the pinned candidate worktree `/private/tmp/rb-sniff-cand-review1` was removed after the measurement; the preserved baseline worktree and its build directories were left untouched
- work completed for 'High — the recorded comparison does not prove the faster-tests outcome' at 16:54:10

### Successful Completion

The implementation of review cycle 1 has completed successfully in 1 hour and
50 minutes. During this implementation all 3 review findings were evaluated to
see if they could be fixed as a part of this implementation cycle: 3 were
fixed, 0 were deferred — but one *clause* inside the second High finding is
deferred, and one finding closed with a negative result that Ken needs to rule
on:

- **Deferred clause — "complete the matched CI samples and ratify the
  per-family timing budgets"** (from *High — The recorded comparison does not
  prove the faster-tests outcome*)
        - per-family budgets are defined against CI runner cells and need three
          consecutive candidate runs per declared environment, which needs the
          branch pushed to `origin`
        - pushing is human-gated and this session is non-interactive, so it
          cannot obtain the credentials a push requires; no local substitute
          exists, because a budget ratified on one macOS host is not a
          compatible sample for the Ubuntu, native-Windows, and WSL2 cells
        - the reason is **not** host CPU load — the local half of this same
          finding was measured successfully, so `deferred_perf_measurement`
          stays `false`; full detail and a close-out procedure are in
          [`deferred-performance-measurement.md`](deferred-performance-measurement.md)

- **Negative result requiring a ruling, not a deferral** — the alternating
  measurement the review demanded was run to completion on clean, pinned trees,
  and it does **not** demonstrate a faster suite. Review-1's finding stands.
  This cycle can report the fact but cannot decide what follows from it: whether
  the fix is accepted on its correctness and isolation merits, whether the
  committed measurement artifacts should be pruned or moved out of the tracked
  tree to recover the `sanity` cohort, or whether further optimization work is
  wanted. That is Ken's call.

The files changed by this cycle are `sniff/cli/tests/common/mod.rs`,
`sniff/cli/tests/cli_process_fixture.rs`, `sniff/cli/tests/spawn_site_guard.rs`,
`sniff/cli/tests/cli.rs`, `sniff/cli/tests/install_interview_cli.rs`,
`sniff/justfile`, `.claude/skills/sniff/SKILL.md`, and this fix's
`results.md`, `log.md`, `deferred-performance-measurement.md`, and
`measurement/review1-alternating/`.
