---
area: claudine
status: ready for review
created: 2026-08-01
packages:
    - claudine-cli
review_iterations: 3
reviewed: true
implemented: true
reviewed_by: claude/default
reviewed_on: 2026-09-06
coordinates_with:
    - claudine/fixes/2026-08-31-silent-success-and-startup-stall/spec.md
---

# Claudine CLI slow tests: retire ambient launch context from the L1 wrapper suite

## Reference environment

CI is the reference, not a developer machine. Every number in this
specification that is not explicitly labeled "local" comes from the
`claudine-cli` L1 JUnit artifacts of the latest successful `main` run
(`33440897014`, 2026-08-31, PR #66 merge) under `NEXTEST_PROFILE=ci`. The
same artifacts for the two preceding `main` runs (`33238966735`,
`33209974555`) agree within a few percent. Pre-fix artifacts from July have
expired, so [inventory.md](inventory.md) remains the record of the
2026-08-01 state.

CI conditions are the normal operating conditions for this suite and the
spec treats them as such:

- Hosted runners are small: `ubuntu-latest` and `windows-latest` are 4 vCPU
  per GitHub's published sizes, `macos-latest` is 3 vCPU, and `wsl2-ubuntu`
  is a guest inside a `windows-latest` runner, sharing its 4 vCPU with the
  host and reading its workspace through the Windows disk.
- The `ci` profile pins `claudine-cli` L1 to the `claudine-cli-ci-l1` group
  with `max-threads = 1`, so per-test cost is CI wall clock one for one.
- Heavy CPU and I/O contention is expected on every run. A test that is
  "close to 5 s" is a slow test.

For scale: the same 2,380 tests run locally on a 16-core Mac under load have
a median duration 0.6× the Ubuntu runner's, and the spawn-shaped tests that
this fix targets run 20–50× slower on WSL2 than locally. Local runs are for
attributing cost (`--perf`), never for proving a target.

## Outcome

Every Level 1 test in `claudine-cli` that spawns the `claudine` binary must run
that process against a workspace the test built, never against the checkout
it was compiled from. This holds by construction (one shared builder) rather
than by per-test environment chains, so a new test is hermetic unless it
explicitly opts out.

Timeout-shaped tests pay only their semantic floor: the configured budget, one
watchdog tick, and termination, with budgets sized for CI contention rather
than whole seconds inherited from before fractional timeout grammar existed.

Targets, per CI environment, on three consecutive `main` or PR runs:

| Measure (per environment) | Ubuntu baseline | WSL2 baseline | Target (Ubuntu / macOS / Windows) | Target (WSL2) |
|---|---:|---:|---:|---:|
| Migrated tests at or over 5 s | 6 | 89 | 0 | 0 |
| Migrated tests at or over 2 s (non-timeout) | 88 | 88 | 0 | ≤ 10 |
| Slowest migrated non-timeout test | 14.3 s | 77.1 s | ≤ 1.5 s | ≤ 4 s |
| Timeout-shaped tests, each | 3.2 s | 3.3 s | ≤ budget + tick + 1 s | ≤ budget + tick + 2 s |
| Inventoried 19 binaries, serial sum | 237.9 s | 1207.3 s | ≤ 80 s | ≤ 300 s |
| Other 10 spawn-shaped binaries, serial sum | 158.7 s | 846.7 s | ≤ 50 s | ≤ 200 s |

"Migrated tests" means every test in the 29 binaries named in Required
behavior 2. "Non-timeout" excludes the nine timeout-shaped tests enumerated in
Required behavior 4; every other migrated test is measured against the
non-timeout rows, so the two populations partition the suite with no test
falling outside both. The four-environment baseline — including macOS and
Windows, and the per-environment ≥ 2 s counts — is the table in
[inventory.md](inventory.md) "2026-09 follow-up → Baseline", computed from the
run `33440897014` artifacts by the measurement script named in Required
behavior 6.

Two reading rules for the Windows column. Only 17 of the 161 inventoried tests
compile on `windows-latest` (the rest are `#[cfg(unix)]`-gated for HOME-shape
or shell reasons that predate this fix), so its serial sum is not comparable
with the other environments' and its target is "no regression plus the ≥ 5 s /
≥ 2 s rules for whatever compiles". Un-gating those tests is out of scope; this
fix must not change any `#[cfg]` gate to make a Windows number look better.

## Problem

The 2026-08-01 pass recorded in [inventory.md](inventory.md) landed on `main`
as `68e2f6db4` (pin CWD and disable rendezvous reporting on the slow tests),
`3520792d4` (run `mcp check`, `prompt_reporting`, and `sequence_perf` from the
isolated workspace), `0810c3c21` (self-contained feature-review contract test),
and `76a68d08c` (retired-flag helper). Every one of the 61 listed tests was
over 5 s on CI at inventory time; that was the inclusion criterion. The pass
brought those 61 under 5 s on the native runners, and this branch has no test
changes on top of it. Three problems remain.

### 1. Isolation was applied per crossing test, not per binary

The pass fixed exactly the tests that had crossed. Their siblings in the same
files still spawn `claudine` from the ambient working directory, which under
`cargo nextest` is the package directory `claudine/cli` inside the monorepo.
On CI those siblings are now the slow tests:

| Environment | 19 inventoried binaries (161 tests) | ≥ 5 s | ≥ 2 s | slowest | 10 other spawn-shaped binaries (92 tests) | ≥ 5 s | slowest |
|---|---:|---:|---:|---:|---:|---:|---:|
| `ubuntu-latest` | 237.9 s serial | 4 | 57 | 8.0 s | 158.7 s | 2 | 14.3 s |
| `macos-latest` | 178.2 s | 2 | 54 | 5.3 s | 118.4 s | 1 | 9.6 s |
| `windows-latest` (17 of 161 compile) | 4.9 s | 0 | 0 | 1.1 s | 63.5 s (60 tests) | 4 | 8.8 s |
| `wsl2-ubuntu` | 1207.3 s | 52 | 57 | 51.0 s | 846.7 s | 37 | 77.1 s |

The tests over 5 s on Ubuntu are all dry runs or fail-before-launch runs that
never execute a provider:
`compose_initialize_error_with_failure_raise_surfaces_failure_evaluation_error`
(8.0 s), `compose_initialize_when_evaluation_error_exits_non_zero` (7.9 s),
`compose_dry_run_quiet_and_silent_are_no_op` (7.2 s),
`inline_compose_dry_run_quiet_and_silent_are_no_op` (7.5 s),
`headline_compose_with_fuzzy_provider_resolves_to_claude` (7.8 s), and
`agents_and_commands_route_to_empty_state_messages` (14.3 s). On WSL2 the
same shape costs 20–51 s per test; the direct-wrapper dry-run helper in
`wrap_compose_agent.rs` is representative at 22–24 s per provider with an
empty `PATH`, which proves the cost is not the provider.

Textual counts of spawn sites against `current_dir(` pins show why: no file
is isolated by default.

| Binary | Spawn sites | CWD pinned | Rendezvous off |
|---|---:|---:|---:|
| `wrap_basics` | 26 | 6 | 0 |
| `mcp_cli` | 18 | 10 | 0 |
| `wrap_compose_validation` | 16 | 3 | 3 |
| `wrap_inline_compose` | 14 | 5 | 5 |
| `wrap_opencode` | 15 | 8 | 5 |
| `argv_normalization` | 14 | 0 | 0 |
| `command_routing` | 11 | 0 | 0 |
| `wrap_compose_agent` | 7 | 3 | 1 |
| `wrap_opencode_models` | 7 | 1 | 1 |
| `wrap_compose_preflight` | 6 | 3 | 1 |
| `wrap_provider_flags` | 5 | 0 | 0 |

### 2. The cost is the monorepo, multiplied by contention

Running the built binary by hand (local, 16-core Mac) with `--perf` attributes
the single-process cost:

| Command | CWD | Total | Dominant substage |
|---|---|---:|---|
| `claudine claude --dry-run hello` | `claudine/cli` | 309 ms | launch discovery 226 ms; system prompt compose 77 ms |
| `claudine claude --dry-run hello` | empty temp dir | 9 ms | none |
| `claudine compose --claude --dry-run prompt.md` | `claudine/cli` | 260 ms | frontmatter load 222 ms (repo-structure detection) |
| `claudine compose --claude --dry-run prompt.md` | empty temp dir | 37 ms | none |

Launch discovery is `LaunchContext::from_cwd`, one `sniff::detect_with_plan`
requesting git summary plus repository structure over the 35-member workspace.
Composition pays the same walk again through `GitRepo::discover` plus
`detect_repo_structure` in `composition/resolve.rs`. The direct wrapper runs
launch discovery (stage 4 in `wrap/mod.rs`) before the dry-run seam, so
`--dry-run` does not avoid it. A quarter second of tree walking on an idle
16-core machine becomes 7–14 s on a contended 4-vCPU runner and 20–77 s in a
WSL2 guest whose workspace lives on the Windows disk.

The ambient CWD is also a correctness hazard, not only a cost. A dry run from
`claudine/cli` discovers and composes the checkout's root `system-prompt.md`
(verified with `--verbose`: `source: <checkout>/system-prompt.md (repo)`),
sees the checkout's git state, and, when a test uses `augmented_path`, can see
the host's real provider binaries during provider selection. Assertions that
pass today pass against a snapshot of whichever machine ran them.

### 3. Timeout tests carry whole-second floors

The six `wrap_watchdog_timeout` tests set `CLAUDINE_STEP_TIMEOUT=2s`,
`CLAUDINE_WATCHDOG_INTERVAL=1s`, and `CLAUDINE_KILL_GRACE=1s`. Their floor is
budget plus up to one tick plus termination: 2.2–3.3 s each on every CI
environment, 15 s serial for the binary, and within one contended tick of
the 5 s line. `sequence_per_step_step_timeout_override` already uses a 0.1 s
tick but keeps a 1 s step budget (1.5–2.0 s on CI). The timeout grammar
accepts decimals (`0.5s` parses; `500ms` does not), so nothing in production
forces whole seconds.

### 4. One contract test composes a prompt that lives in the monorepo

`feature_review_cli_preserves_numeric_iteration_and_dependent_paths` is the
slowest of the 61 listed tests on every environment (4.1 s Ubuntu, 3.1 s
macOS, 21.6 s WSL2). It already runs from an isolated workspace; the cost is
that its subject, `prompts/_reviews/feature-review.md`, sits inside the
monorepo, so `derive_request_context_for_source` anchors repository discovery
on the prompt's directory and walks the whole workspace.

### 5. The ambient *environment* leaks as well as the ambient directory

Pinning the CWD and the home variables does not by itself make a spawn
hermetic, because a child process inherits the parent's whole environment by
default. Three families reach `claudine` today and change what it does:

- **`CLAUDINE_*`.** Forty-nine variables are read across `lib/src` and
  `cli/src`, including `CLAUDINE_STEP_TIMEOUT`, `CLAUDINE_TIMEOUT`,
  `CLAUDINE_WATCHDOG_INTERVAL`, `CLAUDINE_KILL_GRACE`, `CLAUDINE_YOLO`,
  `CLAUDINE_OPTIONS`, and `CLAUDINE_INTERACTIVE`. A developer who exports any
  of them in the shell that runs `just test-cli` silently re-parameterizes the
  timeout tests this fix is tightening.
- **`GIT_*`.** `GIT_DIR`/`GIT_WORK_TREE`/`GIT_INDEX_FILE` override
  cwd-based repository discovery, so an inherited pair defeats `current_dir`
  entirely. This is not hypothetical: on 2026-08-31 a pre-push hook run of the
  workspace suite inherited `GIT_DIR` and drove fixture `git` commands into the
  real repository, committing fixture files onto a feature branch. The hook now
  unsets the family, but nothing in the test suite does.
- **Rendering inputs.** `claudine/cli/src/log.rs` derives its output width from
  `TERM_WIDTH`, then `COLUMNS`, before falling back to 80, and honors
  `FORCE_COLOR`. Every snapshot and every "row is styled" assertion in the
  suite therefore depends on the parent's environment; the same test can wrap
  at a different column on a developer's terminal than on a runner.

This is the same defect as the ambient CWD — isolation left to the call site —
and it is cheapest to close in the same builder.

## Root cause

There is no default-hermetic seam. `common::CliProcessFixture` already provides
the right shape (temp `cwd`, `home`, `bin`; `HOME`/`USERPROFILE`/`APPDATA`
isolation; fake-only `PATH`; `CLAUDINE_RENDEZVOUS_REPORT=false`; `NO_COLOR=1`)
but only five test files use it. Everything else builds
`assert_cmd::Command::cargo_bin("claudine")` inline and re-derives the
environment by hand, so isolation is a property of individual tests rather
than of the suite. The 2026-08-01 pass was threshold-driven and could not
change that; under CI contention the threshold simply moved to the next test.

## Required behavior

### 1. One hermetic builder for every L1 spawn of `claudine`

- Extend `CliProcessFixture` (or split a thin command builder out of it) so it
  is the only supported way an L1 test in `claudine/cli/tests` obtains a
  `claudine` command. Defaults: `current_dir` pinned to the fixture workspace;
  `HOME`, `USERPROFILE`, `APPDATA`, `LOCALAPPDATA` pointed at the fixture home
  with `HOMEDRIVE`, `HOMEPATH`, and `XDG_CONFIG_HOME` removed;
  `CLAUDINE_RENDEZVOUS_REPORT=false`; `NO_COLOR=1`.
- Default `PATH` is the fixture `bin` directory followed by a minimal platform
  system set: `/usr/bin:/bin` on Unix, `%SystemRoot%\System32` on Windows,
  with `PATHEXT` left intact so `.cmd` stubs resolve. This is the decided
  rule (Ken, 2026-09-06), chosen because claudine spawns `sh` and `cmd` by
  bare name for lifecycle shell actions, sequence shell tasks, and
  darkmatter's `$SHELL` alias expansion, so a fake-only `PATH` would break
  every shell-shaped test inside claudine rather than in the fixture, while
  host provider installs live under Homebrew, npm, cargo, and `~/.local/bin`
  prefixes that this set excludes.
- The minimal set resolves a *different* roster per platform, and the docs on
  the builder must say so rather than implying one list. On Unix it resolves
  `sh`, `cat`, `sleep`, and `git`; on Windows `%SystemRoot%\System32` resolves
  `cmd.exe`, `where.exe`, and PowerShell, and resolves **none** of `sh`,
  `cat`, `sleep`, or `git` — Git for Windows installs under `Program Files`,
  which the set deliberately excludes. A fixture script that needs a POSIX
  utility is therefore Unix-gated, rewritten with an absolute path, or given a
  stub in the fixture `bin`; it may not assume the Unix roster on Windows.
- The `SystemRoot` lookup has a fallback (`C:\Windows`) so the default `PATH`
  is well-formed even when a call site has cleared the environment.
- Two named escapes exist, each requiring a comment at the call site that
  names the tool or proof it needs: a fake-only `PATH` for tests whose
  assertion is that nothing else was found (for example the direct-wrapper
  dry-run helper's "an empty `PATH` proves dry-run never resolved the
  provider"), and the full host `PATH` (today's `augmented_path`) for tests
  that need a tool outside the minimal set. The 17 files that currently set
  no `PATH` at all and inherit the test runner's environment are migrated to
  the default; none may keep inheriting.
- Ambient-context behavior must be a *choice*, so the builder offers explicit
  escapes for the few tests whose subject is the launch context:
  `ctx_launch_anchor.rs`, `propagated_context_fixtures.rs`, and any test that
  asserts on repository discovery from a nested directory (for example
  `mcp_default_repo_uses_repo_root_from_nested_directory`). Those tests build
  their own repository inside the workspace with `initialize_repository` or
  `create_claudine_monorepo` and pin the CWD to it; none inherits the
  rusty-biscuit checkout.
- The ambient-context escape enforces that at run time rather than by
  convention: it canonicalizes the requested directory and panics when the
  directory does not exist or resolves outside the fixture workspace. A test
  cannot reach the checkout through the escape even by accident, which is what
  makes "hermetic by construction" true of the escape as well as the default.
  The directory is pinned as the caller spelled it, not in canonical form, so
  a test asserting on paths in claudine's output does not have to know about
  macOS's `/var` → `/private/var` symlink.
- **Environment inheritance is part of the contract, not left to `Command`'s
  default.** In addition to the variables above, the builder removes the
  inherited `CLAUDINE_*` namespace and the `GIT_DIR`/`GIT_WORK_TREE`/
  `GIT_INDEX_FILE`/`GIT_COMMON_DIR`/`GIT_OBJECT_DIRECTORY` plumbing family,
  and pins the rendering inputs `TERM_WIDTH`, `COLUMNS`, and `FORCE_COLOR` to
  fixed values (or removes them, so claudine's documented 80-column fallback
  applies) alongside `NO_COLOR=1`. Removal is per key at build time, so a call
  site that sets `CLAUDINE_STEP_TIMEOUT` after taking the builder still wins —
  the rule scrubs what was *inherited*, never what a test chose. See Open
  question 1 for the deny-list-versus-`env_clear` fork and why the deny list is
  the recommendation.
- A tightening knob is not an escape and does not require the call-site
  comment the escapes do. Tests that assert on what claudine *found* in its own
  environment — the removed-sensitive-variable report is the example — may ask
  the builder for a fully cleared environment on top of the defaults. The
  builder documents what a cleared environment costs (`TERM`, `SystemRoot`,
  `COMSPEC`, and the temp-dir variables all disappear, and on Windows the
  console host needs some of them back to launch a `.cmd` stub) so the call
  site re-adds what its run needs.
- The builder does not change what a test asserts. Real-binary boundary,
  provider stubs, parser paths, and termination behavior stay exactly as
  documented in [inventory.md](inventory.md) "Results".

### 2. Migrate every spawn-shaped L1 binary

- The nineteen inventoried binaries: `compose_header_first`,
  `inline_compose_hash`, `mcp_cli`, `prompt_reporting`, `sequence_perf`,
  `sequence_schema`, `shipped_prompt_contract`, `wrap_compose_agent`,
  `wrap_compose_exec`, `wrap_compose_preflight`, `wrap_compose_validation`,
  `wrap_direct_argv`, `wrap_inline_compose`, `wrap_inline_compose_interactive`,
  `wrap_opencode`, `wrap_opencode_models`, `wrap_perf`,
  `wrap_structured_stream`, `wrap_watchdog_timeout`.
- The ten other binaries whose tests are over 5 s on at least one CI
  environment for the same reason: `wrap_basics`, `wrap_provider_flags`,
  `command_routing`, `argv_normalization`, `hooks_cli`, `contextual_errors`,
  `inline_compose_sequence_mismatch`, `wrap_antigravity_exit_signal`,
  `handle_repo_config`, `characterization_error_routes`.
- Every spawn site in those 29 binaries moves to the builder, including tests
  the 2026-08-01 pass already isolated, so each file ends with one spawn
  idiom rather than two. For those tests the diff should read as deletions of
  repeated `.env(...)` chains and nothing else.
- Fixture scripts that call system utilities by bare name (`sleep 30` in
  `wrap_opencode.rs` and `sequence_schema.rs`) either keep working under the
  minimal system `PATH` or switch to absolute paths as neighboring fixtures
  already do.
- **Scope boundary.** Twenty-nine binaries are in; roughly thirty-five other L1
  files also spawn `claudine` raw (`compose_cli.rs`, `sequence_*.rs`,
  `context_command.rs`, `loop_cli.rs`, `skills_integration.rs`, the
  `completion_*` trio, and the rest). They are *not* migrated here — the
  inclusion rule is "at or over 5 s on at least one CI environment for the
  ambient-context reason", and widening it would make one change set touch
  most of the suite. They are held by the guard's allow-list instead
  (Required behavior 3), and the residual census — file count, site count — is
  recorded in [inventory.md](inventory.md) so the follow-up fix that finishes
  the burn-down starts from a number rather than a re-survey. That follow-up is
  named in Out of scope.

### 3. A spawn-site guard, in the style of the existing structural gates

- Add a Level 1 guard beside `test_placement.rs` and `dispatch_inventory.rs`
  that scans `claudine/cli/tests` and fails when a file constructs
  `assert_cmd::Command::cargo_bin("claudine")` or shells out to
  `claudine_bin()` outside the builder. It sanitizes comments and string
  literals before searching, as `test_placement.rs` does, so a mention in prose
  is not a site.
- Four exclusions, each for a stated reason: `level2_*`, `level3_*`, and
  `real_*` files drive real terminals, real providers, and real host tooling on
  purpose — the hermetic default is the wrong contract for them, and
  `claudine_bin()` remains the supported path there — and `common/` is where
  the builder itself lives. The guard governs L1 only; it does not make
  `claudine_bin()` deprecated.
- Any file not migrated is listed in an explicit allowlist with a one-line
  reason, and the guard also fails when an allowlist entry no longer matches
  a live site, so the list cannot become a grandfather table.
- **The end state is a burn-down list, not an empty one.** The allowlist ends
  this fix holding the out-of-scope files from Required behavior 2 — each with
  the reason "outside this fix's scope" — and naming **no** file among the 29
  and no ambient-context subject; that, not emptiness, is the testable
  condition (acceptance criterion 1). *Reader's note: an earlier draft required
  the list to be empty. That is unreachable without migrating the whole suite
  in one change, and a requirement no implementation can satisfy is worse than
  none: it invites either silent scope creep or a quietly weakened guard. The
  burn-down framing keeps the pressure — the list only ever shrinks, and a
  stale entry fails the build — while leaving the remainder to a follow-up.*
- The guard's failure output names files, lines, and forms, and prints the
  roll-up (files and sites still allow-listed) so the burn-down is observable
  from a CI log without re-running a census.

### 4. Sub-second floors for timeout-shaped tests, sized for CI

- `wrap_watchdog_timeout`: `CLAUDINE_WATCHDOG_INTERVAL` between `0.1s` and
  `0.2s`; `CLAUDINE_STEP_TIMEOUT` no lower than `1s` for the hang tests and
  never lower than four times the largest gap the fixture can have between
  its own pre-hang writes under WSL2 contention; `CLAUDINE_KILL_GRACE` no
  higher than `0.5s` where the fixture exits on `TERM`. The comment on each
  budget states the margin it keeps and why.
- `watchdog_opencode_post_fanout_silence_does_not_kill_prematurely` keeps its
  built-in one-second silence, because "silence shorter than the budget must
  not kill" is the contract; only its tick and grace shrink.
- `sequence_per_step_step_timeout_override`: the step-level budget drops to
  `0.5s`; the document-level `30s` fallback and the "fires well before the
  fallback" assertion stay. The elapsed assertion tightens to a bound that
  still fails if the fallback ever wins.
- `wrap_opencode` early-termination tests keep their `sleep 30` fixtures (the
  point is that Claudine, not the sleep, ends the run) but adopt the same tick
  and grace so the abort path is not waiting on a one-second tick.
- Budgets are proven on CI, not locally: each timeout test must pass on all
  four environments across three consecutive runs, and its CI duration must
  sit within budget + tick + 1 s (Ubuntu, macOS, Windows) or + 2 s (WSL2). A
  budget that needs raising to pass on WSL2 is raised for every environment;
  no per-environment budgets.
- No new nextest `slow-timeout` overrides. Raising a limit hides cost; this
  fix removes cost.

### 5. The shipped feature-review contract test

- Copy the `prompts/` corpus (45 Markdown files) into the fixture workspace
  so the relative `::file ../_senior-reviewer.md` reference and the
  `@{{spec}}` file references resolve inside an isolated repository. The test
  still exercises the shipped content byte-for-byte; only its location moves.
- Confirm with `--perf` (local, attribution only) that repository discovery
  no longer appears in the profile, and with CI durations that the test drops
  under the migrated-test target on every environment.

### 6. Measurement and record

- CI JUnit artifacts (`junit-claudine-cli-L1-<environment>`) are the
  measurement of record. Record in a new "2026-09 follow-up" section of
  [inventory.md](inventory.md), per environment and for the baseline run
  named above versus the first three green runs after the change: count at
  or over 5 s, count at or over 2 s, slowest test, and serial sum for the
  inventoried 19 and the other 10 binaries.
- Local `--perf` output of one migrated dry-run test is recorded as the
  attribution proof (launch discovery under 5 ms, no repository system
  prompt), labeled local.
- A run that misses a target is reported as a miss with its cause, not
  adjusted to fit.

### 7. Documentation

- `claudine/cli/tests/common/mod.rs` module docs describe the builder as the
  L1 spawn contract and name the opt-outs.
- The `rust-testing` skill and the Claudine skill's testing notes point at
  the builder and the guard, in the same change.

## Out of scope

The inventory's production-concerns catalog stands, with sharper evidence from
this review. None of it is changed here; each item is a candidate for its own
fix spec:

1. **Launch discovery cost and placement.** About 220 ms per process from a
   35-member workspace on an idle machine, paid by direct wrappers before the
   `--dry-run` seam and again by composition's repo-structure detection.
   Candidates: a narrower sniff plan, one shared walk per invocation, and
   moving the direct wrapper's discovery after the dry-run seam. Users on
   large repositories pay this on every launch; the tests merely made it
   visible.
2. **Root `system-prompt.md` composition** adds 77 ms to every wrapper launch
   from this checkout and belongs in the user-visible startup budget.
3. **Watchdog cadence.** The 5 s default tick means a 1 s `step_timeout` fires
   at about 5.4 s. The 2026-08-31 startup-stall spec
   (`claudine/fixes/2026-08-31-silent-success-and-startup-stall/spec.md`)
   changes the silence clock's origin, and its acceptance criterion 1 assumes
   "within `step_timeout` plus one watchdog interval"; the budgets chosen in
   Required behavior 4 must stay valid under that rule. Coordinate, do not
   merge the two efforts.
4. **Rendezvous reporting default-on** stays bounded at 250 ms per call and
   was not the dominant cost.
5. **Stale identity of `compose_opencode_dry_run_calls_opencode_models_and_fails_with_test_double`**
   and the tautological `row["error"]` comparison remain test-maintenance
   items to land as their own comment-only or assertion-only commits.
6. **Relative `spec=` reference inconsistency** between `file_exists` and
   `frontmatter` needs separate diagnosis; Required behavior 5 keeps using an
   absolute spec path.

Slow CI tests that are not spawn-shaped are out of scope and need their own
inventory: `context_reports_preserve_all_columns_at_minimum_supported_width`
(13.0 s Ubuntu, 13.4 s macOS, 11.0 s Windows, 63.8 s WSL2, already carrying a
30 s override), `context_footer_no_availability_claims` (5.9 s macOS), the
twelve `error_guards` source scans (2–4 s each on Ubuntu), the `loop_cli`
pause tests, and the in-crate `bin/claudine` unit tests over 2 s.

## Decisions taken

**`PATH` default: fixture bin plus a minimal system set (Option B).** Three
options were weighed on 2026-09-06. Fake-only (today's
`CliProcessFixture::command`) is the strictest but breaks every test that
runs a shell inside claudine on every platform, so the opt-in would become
the norm. Full host `PATH` (today's `augmented_path`, 209 sites in 34 files)
keeps host providers visible to sniff's `which`-based discovery and so keeps
the machine-dependence this fix removes. The minimal system set resolves
`sh`, `cmd`, `cat`, `sleep`, and `git` while excluding every prefix a
provider installs into, and leaves fake-only and full-host as explicit,
commented escapes. Required behavior 1 carries the rule.

## Acceptance criteria

1. **Builder is the sole spawn path.** All 29 binaries in Required behavior 2
   spawn `claudine` only through the builder; the guard test passes with an
   allowlist that names no migrated file.
2. **Guard is non-vacuous.** Adding a raw `cargo_bin("claudine")` spawn to a
   migrated file makes the guard fail; removing a live allowlist entry's
   matching site makes the guard fail; both are demonstrated and reverted
   before the final run.
3. **Isolation is proven, not inferred.** One migrated dry-run test, run
   locally with `--perf`, reports launch discovery under 5 ms and no
   repository system prompt. The same test run from a checkout whose root
   `system-prompt.md` has been modified produces identical output.
4. **CI targets are met** on `ubuntu-latest`, `macos-latest`,
   `windows-latest`, and `wsl2-ubuntu` for three consecutive green runs, as
   read from the JUnit artifacts: zero migrated tests at or over 5 s on any
   environment; zero non-timeout migrated tests at or over 2 s on the native
   runners and at most ten on WSL2; slowest non-timeout migrated test at or
   under 1.5 s native and 4 s WSL2; serial sums at or under the Outcome
   table. The numbers are written to [inventory.md](inventory.md).
5. **Assertions unchanged.** Every migrated test keeps its original
   assertions; the diff for previously isolated tests contains only
   environment-setup deletions. Fixture scripts change only where a bare
   utility name needed an absolute path.
6. **Timeout budgets are justified and CI-proven.** Each shortened budget
   carries a comment stating the margin it keeps; each timeout test's CI
   duration sits within budget + tick + 1 s (native) or + 2 s (WSL2) with
   zero failures across the three runs; locally, each timeout test is run
   ten times in a row under full `just test-cli` load with zero failures.
7. **No slow-timeout overrides added** to `.config/nextest.toml` for any test
   touched by this fix.
8. **Cross-platform.** The builder's environment handling is exercised on all
   four CI environments; the Windows home shape follows what
   `CliProcessFixture` already does, and the default `PATH` is the minimal
   system set from Required behavior 1 on every platform. A guard proves the
   default: a migrated test that spawns claudine with a fake provider named
   like a host-installed one resolves the fake, and a probe for a Homebrew
   or npm prefix on the child's `PATH` finds none. No terminal or browser
   window gains focus.
9. **Verification through the canonical recipes.** `just test-cli`,
   `just lint`, and `just ci-local` pass from the `claudine` package area;
   `just test-l2` passes for any L2 binary whose helpers were touched. No
   `cargo test`.
10. **Docs and skills** in Required behavior 7 land in the same change.
