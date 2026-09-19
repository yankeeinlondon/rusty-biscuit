---
total_phases: 7
created: 2026-09-06
phase: 7
agent: opencode/zai-coding-plan/glm-5.3
yolo: "true"
packages:
  - claudine-cli
source_files_during_phase_1:
  - claudine/cli/tests/common/mod.rs
  - claudine/cli/tests/cli_process_fixture.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/cli/tests/spawn_site_guard.rs
  - claudine/cli/tests/common/source_scan.rs
  - claudine/cli/tests/common/mod.rs
  - claudine/cli/tests/test_placement.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/cli/tests/compose_header_first.rs
  - claudine/cli/tests/inline_compose_hash.rs
  - claudine/cli/tests/wrap_compose_agent.rs
  - claudine/cli/tests/wrap_compose_exec.rs
  - claudine/cli/tests/wrap_compose_preflight.rs
  - claudine/cli/tests/wrap_compose_validation.rs
  - claudine/cli/tests/wrap_inline_compose.rs
  - claudine/cli/tests/wrap_inline_compose_interactive.rs
  - claudine/cli/tests/prompt_reporting.rs
  - claudine/cli/tests/shipped_prompt_contract.rs
  - claudine/cli/tests/wrap_opencode.rs
  - claudine/cli/tests/wrap_opencode_models.rs
  - claudine/cli/tests/wrap_perf.rs
  - claudine/cli/tests/wrap_structured_stream.rs
  - claudine/cli/tests/wrap_direct_argv.rs
  - claudine/cli/tests/mcp_cli.rs
  - claudine/cli/tests/sequence_perf.rs
  - claudine/cli/tests/sequence_schema.rs
  - claudine/cli/tests/wrap_watchdog_timeout.rs
  - claudine/cli/tests/spawn_site_guard.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - claudine/cli/tests/common/mod.rs
  - claudine/cli/tests/cli_process_fixture.rs
  - claudine/cli/tests/spawn_site_guard.rs
  - claudine/cli/tests/wrap_basics.rs
  - claudine/cli/tests/snapshots/wrap_basics__wrapper_reports_removed_sensitive_env_names.snap
  - claudine/cli/tests/wrap_provider_flags.rs
  - claudine/cli/tests/command_routing.rs
  - claudine/cli/tests/argv_normalization.rs
  - claudine/cli/tests/hooks_cli.rs
  - claudine/cli/tests/contextual_errors.rs
  - claudine/cli/tests/inline_compose_sequence_mismatch.rs
  - claudine/cli/tests/wrap_antigravity_exit_signal.rs
  - claudine/cli/tests/handle_repo_config.rs
  - claudine/cli/tests/characterization_error_routes.rs
  - claudine/cli/tests/ctx_launch_anchor.rs
  - claudine/cli/tests/propagated_context_fixtures.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - claudine/cli/tests/wrap_watchdog_timeout.rs
  - claudine/cli/tests/sequence_schema.rs
  - claudine/cli/tests/wrap_opencode.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6:
  - claudine/cli/tests/shipped_prompt_contract.rs
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
source_files_during_phase_7:
  - claudine/cli/tests/common/mod.rs
  - claudine/cli/tests/wrap_basics.rs
  - claudine/fixes/2026-08-01-cli-slow-tests/junit-metrics.ts
docs_updated_during_phase_7:
  - claudine/fixes/2026-08-01-cli-slow-tests/inventory.md
  - claudine/fixes/2026-08-01-cli-slow-tests/plan.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
  - .claude/skills/rust-testing/SKILL.md
  - .claude/skills/claudine/SKILL.md
packages:
  - claudine-cli
source_code:
  - claudine/cli/tests/common/mod.rs
  - claudine/cli/tests/common/source_scan.rs
  - claudine/cli/tests/cli_process_fixture.rs
  - claudine/cli/tests/spawn_site_guard.rs
  - claudine/cli/tests/test_placement.rs
  - claudine/cli/tests/argv_normalization.rs
  - claudine/cli/tests/characterization_error_routes.rs
  - claudine/cli/tests/command_routing.rs
  - claudine/cli/tests/compose_header_first.rs
  - claudine/cli/tests/contextual_errors.rs
  - claudine/cli/tests/ctx_launch_anchor.rs
  - claudine/cli/tests/handle_repo_config.rs
  - claudine/cli/tests/hooks_cli.rs
  - claudine/cli/tests/inline_compose_hash.rs
  - claudine/cli/tests/inline_compose_sequence_mismatch.rs
  - claudine/cli/tests/mcp_cli.rs
  - claudine/cli/tests/prompt_reporting.rs
  - claudine/cli/tests/propagated_context_fixtures.rs
  - claudine/cli/tests/sequence_perf.rs
  - claudine/cli/tests/sequence_schema.rs
  - claudine/cli/tests/shipped_prompt_contract.rs
  - claudine/cli/tests/snapshots/wrap_basics__wrapper_reports_removed_sensitive_env_names.snap
  - claudine/cli/tests/wrap_antigravity_exit_signal.rs
  - claudine/cli/tests/wrap_basics.rs
  - claudine/cli/tests/wrap_compose_agent.rs
  - claudine/cli/tests/wrap_compose_exec.rs
  - claudine/cli/tests/wrap_compose_preflight.rs
  - claudine/cli/tests/wrap_compose_validation.rs
  - claudine/cli/tests/wrap_direct_argv.rs
  - claudine/cli/tests/wrap_inline_compose.rs
  - claudine/cli/tests/wrap_inline_compose_interactive.rs
  - claudine/cli/tests/wrap_opencode.rs
  - claudine/cli/tests/wrap_opencode_models.rs
  - claudine/cli/tests/wrap_perf.rs
  - claudine/cli/tests/wrap_provider_flags.rs
  - claudine/cli/tests/wrap_structured_stream.rs
  - claudine/cli/tests/wrap_watchdog_timeout.rs
  - claudine/fixes/2026-08-01-cli-slow-tests/junit-metrics.ts
documentation:
  - claudine/fixes/2026-08-01-cli-slow-tests/inventory.md
  - claudine/fixes/2026-08-01-cli-slow-tests/plan.md
  - .claude/skills/rust-testing/SKILL.md
  - .claude/skills/claudine/SKILL.md
---

# Execution plan: retire ambient launch context from the claudine-cli L1 suite

Source: [spec.md](spec.md) — every requirement below traces to a "Required
behavior" (RB) section or acceptance criterion (AC 1–10) there. Baselines and
targets are the spec's Outcome table; CI JUnit artifacts are the measurement of
record, local `--perf` runs are for attribution only.

## Grounding facts (verified on this branch)

- `claudine/cli/tests/common/mod.rs` holds `CliProcessFixture` (temp
  `cwd`/`home`/`bin`, env isolation, `CLAUDINE_RENDEZVOUS_REPORT=false`,
  `NO_COLOR=1`) whose `command()` is currently **fake-only PATH**; the free
  function `augmented_path()` (273 sites) is the full-host escape; five files
  use the fixture today (`contextual_errors.rs`,
  `propagated_context_fixtures.rs`, `wrap_compose_exec.rs`,
  `wrap_compose_preflight.rs`, plus `common` itself).
- `common/wrap.rs` already provides `seed_minimal_config`,
  `create_claudine_monorepo`, and `initialize_repository` (via the fixture) —
  the ambient-context escape builds on these.
- Structural-gate style to copy: `claudine/cli/tests/test_placement.rs` and
  `dispatch_inventory.rs` (the latter shows the allowlist + stale-entry
  failure pattern to mirror).
- `wrap_watchdog_timeout.rs` (6 tests) sets `STEP_TIMEOUT=2s`,
  `WATCHDOG_INTERVAL=1s`, `KILL_GRACE=1s`; the post-fanout test
  (`watchdog_opencode_post_fanout_silence_does_not_kill_prematurely`) uses a
  built-in 1 s silence and must keep it.
- `sequence_per_step_step_timeout_override` lives in
  `claudine/cli/tests/sequence_schema.rs:255` (step budget `1s`, document
  fallback `30s`, `sleep 30` fixture at line 315).
- `feature_review_cli_preserves_numeric_iteration_and_dependent_paths` lives
  in `claudine/cli/tests/shipped_prompt_contract.rs:163`; the shipped corpus
  is the **repo-root `prompts/` directory (45 `.md` files)**, and
  `repository_root()` resolves it via `CARGO_MANIFEST_DIR/../..`.
- Timeout grammar: decimals parse (`0.5s`), `ms` does not.
- All 29 in-scope binaries exist as `.rs` files under `claudine/cli/tests/`.
- Canonical recipes (from the `claudine` package area unless noted):
  `just test-cli`, `just lint`, `just ci-local` (shared recipe), `just
  test-l2`; never `cargo test`. Root: `just test claudine-cli`.

## Assumptions and stated decisions

1. **Guard-first burndown.** The spawn-site guard (RB 3) lands early with an
   allowlist seeded from the *current* raw-spawn census; every migration task
   deletes its file's entries. Progress is observable (allowlist shrinks) and
   migration-phase regressions fail the suite immediately.
2. **Allowlist end state.** The guard scans all of `claudine/cli/tests`
   (excluding `level2_*`, `level3_*`, `real_*`, and `common/` where the
   builder lives). Files with raw spawn sites that are **not** among the 29
   in-scope binaries nor the named ambient-context tests (e.g.
   `compose_cli.rs`, `sequence_cli.rs`, `skills_integration.rs`,
   `context_command.rs`) stay on the allowlist with a one-line
   "outside this fix's scope" reason. AC 1 is the operative contract: the
   allowlist names **no migrated file**. The spec's "should be empty or hold
   only the ambient-context tests" sentence is treated as aspirational for
   those out-of-scope files; if the implementer prefers, migrating them is a
   compatible superset — do not silently expand scope either way.
3. **Two-phase touch of the three timeout files.** Migration (mechanical,
   assertions unchanged) and budget changes (Phase 5) are kept in separate
   commits/phases so the migration diff reads as pure environment-setup
   deletion, per AC 5.
4. **One fix, many commits.** All phases land on this branch as one change
   set (spec RB 7 requires docs in the same change). Commit splitting follows
   the monorepo's comment-only/comment-discipline rules.
5. CI observation (3 consecutive green runs × 4 environments) is wall-clock
   bound; the plan treats it as the closing gate, started as soon as the code
   is complete.

---

## Phase 1 — The hermetic builder (foundation for everything)

Goal: `common/mod.rs` exposes the one supported way an L1 test obtains a
`claudine` command, with the decided default PATH rule (RB 1) and the named
escapes. Nothing else migrates yet.

- [x] Extend `CliProcessFixture` in `claudine/cli/tests/common/mod.rs` with a
      builder surface whose **default** command sets: `current_dir` pinned to
      the fixture `cwd`; `HOME`/`USERPROFILE`/`APPDATA`/`LOCALAPPDATA` →
      fixture `home`; `HOMEDRIVE`/`HOMEPATH`/`XDG_CONFIG_HOME` removed;
      `CLAUDINE_RENDEZVOUS_REPORT=false`; `NO_COLOR=1`;
      **`PATH` = fixture `bin` + minimal system set** — `/usr/bin:/bin` on
      Unix, `%SystemRoot%\System32` on Windows, `PATHEXT` left intact so
      `.cmd` stubs resolve (decided rule, spec "Decisions taken").
- [x] Add the named escapes, each documented as requiring a call-site comment
      naming the tool or proof it needs:
      - fake-only `PATH` (today's `command()` semantics) — for tests whose
        assertion is that nothing else was found;
      - full host `PATH` (today's `augmented_path`) — for tests needing a
        tool outside the minimal set;
      - ambient-context escape — CWD pinned to a repository the test built
        inside the workspace via `initialize_repository` /
        `create_claudine_monorepo`; the checkout is never inherited.
- [x] Audit the five existing `CliProcessFixture` users
      (`contextual_errors.rs`, `propagated_context_fixtures.rs`,
      `wrap_compose_exec.rs`, `wrap_compose_preflight.rs`) for reliance on the
      old fake-only default; switch any that do to the fake-only escape with
      the required comment, keep the rest on the new default.
- [x] Add builder self-tests (a new L1 test file): child-process env shape is
      asserted via a spawned `claudine` probe (or fixture stub) — on all
      platforms present locally, `#[cfg]`-gated where OS-specific; a fake
      provider named like a host-installed one resolves to the **fake**; a
      probe for a Homebrew/npm/cargo prefix on the child's `PATH` finds
      **none** (AC 8's guard, runnable without a terminal).
- [x] Write the `common/mod.rs` module docs: the builder is the L1 spawn
      contract; the default PATH rule and why fake-only would break
      shell-shaped tests; the three opt-outs and their call-site-comment
      requirement (RB 7, first half).

Validation (Phase 1 checkpoint):

- [x] `just test-cli` green from the `claudine` package area — no existing
      test changed behavior; the five audited files still pass.
      (2389 passed / 10 skipped, 24.9 s.)
- [x] `just lint` green.
- [x] The new self-tests prove the PATH default and env isolation on macOS
      (this host); Windows-shaped assertions are compile-checked via `#[cfg]`
      and exercised by CI legs. **Caveat:** the workspace cannot cross-compile
      to Windows (aws-lc-sys / duckdb-sys), and this branch has no
      `just cross-check` recipe, so the `#[cfg(windows)]` branches of the new
      self-test file are first compiled on the `windows-latest` CI leg.

## Phase 2 — Spawn-site guard with a burndown allowlist (RB 3)

Goal: the structural gate exists and passes against the *unmigrated* tree, so
every later migration is enforced, not trusted.

- [x] Add a Level 1 guard test beside `test_placement.rs` (e.g.
      `claudine/cli/tests/spawn_site_guard.rs`) that scans
      `claudine/cli/tests` — excluding `level2_*`, `level3_*`, `real_*` files
      and the `common/` builder home — and fails when a file constructs
      `assert_cmd::Command::cargo_bin("claudine")` or shells out to
      `claudine_bin()` outside the builder. Source-scanning follows the
      sanitize-then-search pattern of `test_placement.rs` so sites inside
      comments/strings don't false-positive.
- [x] Implement the explicit allowlist as `(file, one-line reason)` entries
      with **stale-entry failure** (an entry matching no live site fails the
      guard), mirroring `dispatch_inventory.rs`'s GUARD_ALLOWLIST mechanics.
- [x] Seed the allowlist from a fresh census: every non-excluded file with a
      raw `cargo_bin("claudine")` or `claudine_bin()` shell-out site, each
      with its reason ("in-scope, migrates in this fix" for the 29 +
      ambient-context files; "outside this fix's scope" for the rest — see
      Assumption 2).
      (65 files / 401 sites: 30 in-scope / 232 sites, 35 out-of-scope /
      169 sites.)
- [x] Smoke-test non-vacuity now: temporarily delete one seeded entry whose
      site still exists → guard fails with that file named; restore. (The
      full AC 2 demonstration — including adding a raw spawn to a *migrated*
      file — runs in Phase 7.)
      (Deleted the `wrap_basics.rs` entry → guard failed listing its 26 sites
      as `wrap_basics.rs:<line> cargo_bin`; entry restored.)

Validation (Phase 2 checkpoint):

- [x] `just test-cli` green with the seeded allowlist; the guard's failure
      output names files and reasons legibly.
- [x] Guard census counts recorded in the PR description (files + sites)
      match a manual `rg -c 'cargo_bin\("claudine"\)'` sanity check.
      (Manual grep over the same file set: 400 `cargo_bin("claudine")` +
      7 `claudine_bin(` = 407 across 66 files. The guard reports 401 across
      65: the 6-hit delta is prose — 1 module-doc mention in
      `level1_structured_error_message.rs:34` and 5 in the guard's own docs
      and string-literal test fixtures, all of which `sanitize` blanks.)

## Phase 3 — Migrate the 19 inventoried binaries (RB 2, first half)

Goal: every spawn site in the 19 binaries builds its command through the
builder; each file ends with one spawn idiom. **All four batches are
parallelizable with each other** (and with Phase 4) once Phases 1–2 land.

Every migration task follows the same definition of done: all spawn sites use
the builder; the file's allowlist entries are deleted; assertions are
byte-identical in intent; the diff for tests the 2026-08-01 pass already
isolated reads as deletions of repeated `.env(...)` chains and nothing else;
bare-name utilities in fixtures either still resolve under the minimal system
PATH or switch to absolute paths as neighboring fixtures already do.

- [x] **Batch A — compose family:** migrate `compose_header_first.rs`,
      `inline_compose_hash.rs`, `wrap_compose_agent.rs`,
      `wrap_compose_exec.rs`, `wrap_compose_preflight.rs`; drop their
      allowlist entries.
- [x] **Batch B — validation/inline/reporting:** migrate
      `wrap_compose_validation.rs`, `wrap_inline_compose.rs`,
      `wrap_inline_compose_interactive.rs`, `prompt_reporting.rs`,
      `shipped_prompt_contract.rs` (mechanical migration only — the
      feature-review test's corpus move is Phase 6, do not mix it in).
- [x] **Batch C — provider wrappers:** migrate `wrap_opencode.rs`,
      `wrap_opencode_models.rs`, `wrap_perf.rs`,
      `wrap_structured_stream.rs`, `wrap_direct_argv.rs`; drop their allowlist
      entries. (Budget changes in `wrap_opencode.rs` wait for Phase 5.)
- [x] **Batch D — mcp/sequence/watchdog:** migrate `mcp_cli.rs` (including
      `mcp_default_repo_uses_repo_root_from_nested_directory`, which uses the
      ambient-context escape over its own in-workspace repo),
      `sequence_perf.rs`, `sequence_schema.rs` (its `sleep 30` fixture keeps
      working under `/usr/bin:/bin` or switches to an absolute path),
      `wrap_watchdog_timeout.rs` (migration only; budgets are Phase 5); drop
      their allowlist entries.

Validation (Phase 3 checkpoint):

- [x] `just test-cli` green after each batch; guard green with the 19 files
      gone from the allowlist.
      (Final: 2395 passed / 10 skipped, 16.0 s. Guard burn-down now 246 sites
      in 46 files: 11 in-scope files / 77 sites — exactly Phase 4's set —
      and 35 out-of-scope files / 169 sites.)
- [x] Spot-review: `git diff` on previously-isolated tests shows only
      environment-setup deletions (AC 5). Three stale comments explaining why
      those tests pinned CWD ("with the ambient monorepo as cwd … git
      worktree-metadata refresh") went with them: the builder now pins CWD
      universally, so the comments described a choice the call site no longer
      makes. Two assertions needed a non-environment change and are called out
      in the phase report.
- [x] Local timing sanity (attribution only, not a target): a migrated
      dry-run test no longer reports launch-discovery cost in `--perf`.
      (`compose --goose --dry-run --perf` on an equivalent document: from a
      fixture-shaped CWD, 33.6 ms total / prep phase 4.4 ms / `topology
      probes 0`; from the monorepo checkout, 260.9 ms total / prep phase
      230.5 ms / `topology probes 1, topology reuses 1`.)

## Phase 4 — Migrate the 10 other spawn-shaped binaries + ambient subjects (RB 2, second half)

Goal: the remaining ten over-5-s-on-some-environment binaries plus the two
ambient-context subjects are on the builder. **Parallelizable with Phase 3**
and internally across files.

- [x] Migrate `wrap_basics.rs` (26 spawn sites, 6 CWD-pinned today — the
      largest single file) and drop its allowlist entry.
- [x] Migrate `wrap_provider_flags.rs`, `command_routing.rs`, and
      `argv_normalization.rs` — the zero-CWD-pinning files whose tests today
      inherit the runner's environment entirely (the "17 files set no PATH"
      population) — and drop their allowlist entries.
- [x] Migrate `hooks_cli.rs`, `contextual_errors.rs`,
      `inline_compose_sequence_mismatch.rs`,
      `wrap_antigravity_exit_signal.rs`, `handle_repo_config.rs`,
      `characterization_error_routes.rs`; drop their allowlist entries.
- [x] Migrate `ctx_launch_anchor.rs` onto the builder's ambient-context
      escape (its 5 `cargo_bin` sites each pin CWD to a self-built context;
      none may inherit the rusty-biscuit checkout) and drop its allowlist
      entry.
- [x] Verify `propagated_context_fixtures.rs` (already a fixture user)
      expresses its ambient needs through the escape rather than ad-hoc env
      chains; adjust and comment if not.

Validation (Phase 4 checkpoint):

- [x] `just test-cli` green; the guard's allowlist now names **no file from
      the 29** and no ambient-context subject (AC 1's operative test).
      (2396 passed / 10 skipped, 14.2 s. Guard burn-down: 169 sites in 35
      files, all `outside this fix's scope`; zero in-scope entries left.)
- [x] `just test-l2` green (helpers in `common/` are shared with L2 binaries;
      AC 9).
      (230 passed / 2410 skipped for the claudine area, plus 3 passed for
      `claudine-gen`.)
- [x] `git grep -n 'cargo_bin("claudine")' claudine/cli/tests` outside
      `common/` returns only allowlisted, out-of-scope files.
      (50 files match; subtracting the 35 allow-listed leaves only 17
      `level2_*` and 1 `real_*` file, which the guard excludes by tier.)

## Phase 5 — Sub-second timeout floors, sized for CI (RB 4)

Depends on: Phases 3B/3C/3D having migrated `sequence_schema.rs`,
`wrap_opencode.rs`, `wrap_watchdog_timeout.rs`. Independent of Phase 4 files
and Phase 6. Keep this in its own commit(s) so migration diffs stay pure.

- [x] `wrap_watchdog_timeout.rs`: set `CLAUDINE_WATCHDOG_INTERVAL` to
      0.1–0.2 s; `CLAUDINE_STEP_TIMEOUT` no lower than 1 s for the hang tests
      and never lower than 4× the fixture's largest gap between pre-hang
      writes; `CLAUDINE_KILL_GRACE` ≤ 0.5 s where the fixture exits on
      `TERM`. Each budget carries a comment stating the margin it keeps and
      why (spec floor: budget + one tick + termination).
      (All six tests now `1s` budget / `0.2s` tick / `0.5s` grace — the two
      wall-clock tests via `CLAUDINE_TIMEOUT` and `--timeout`. The shared
      derivation is a new module-doc section; each site carries the
      fixture-specific margin. Local: 13.8 s → 8.2 s serial for the binary.)
- [x] `watchdog_opencode_post_fanout_silence_does_not_kill_prematurely`:
      keep the built-in 1 s silence (the contract is "silence shorter than
      the budget must not kill"); shrink only its tick and grace.
      (Budget stays `15s`; tick `2s` → `0.2s`, grace `1s` → `0.5s`.)
- [x] `sequence_per_step_step_timeout_override` (`sequence_schema.rs`): drop
      the step-level budget to `0.5s`; keep the document-level `30s` fallback
      and the fires-well-before-fallback assertion; tighten the elapsed bound
      so the test still fails if the fallback ever wins.
      (Elapsed bound 5 s → 4 s; 1.852 s → 1.209 s locally.)
- [x] `wrap_opencode.rs` early-termination tests: keep the `sleep 30`
      fixtures (the point is Claudine ends the run, not the sleep); adopt the
      same 0.1–0.2 s tick and ≤ 0.5 s grace so the abort path is not waiting
      on a one-second tick.
      (Both tests previously ran on the built-in `5s` tick / `10s` grace and
      set no knobs at all; now `0.2s` / `0.5s`.)
- [x] Cross-check against the 2026-08-31 startup-stall spec (out-of-scope
      item 3): confirm each chosen budget stays valid under "fires within
      `step_timeout` plus one watchdog interval"; note the check in the
      commit message. Coordinate only — do not merge the two efforts.
      (That spec's RB 1 makes the silence reference
      `max(started_at, last_event_at, last_byte_at)`, so `step_timeout` will
      *also* bound startup. Every tick cut here only tightens its AC 1 bound,
      so no budget becomes invalid. Budget-by-budget: the four `1s`
      `step_timeout` tests' fixtures write their first line on exec, so the
      new startup term is shell-spawn latency and `1s` stays far above the
      4× floor; the two wall-clock tests disable `step_timeout` (`0s`)
      entirely, so the startup clock cannot reach them; `post_fanout` keeps
      `15s`. **One coordination flag for the 08-31 effort:**
      `sequence_per_step_step_timeout_override`'s `0.5s` step budget is the
      only value here whose margin shrinks under the new clock — step 2 would
      have to spawn `/bin/sh`, read the counter file, and emit two `printf`s
      inside `0.5s` on a contended WSL2 runner. Not raised here (the spec for
      this fix mandates `0.5s`); re-measure it when the startup clock lands.)
- [x] Confirm **no new** `slow-timeout` overrides in `.config/nextest.toml`
      for any touched test (AC 7); the diff to that file must be empty.
      (`git diff main -- .config/nextest.toml` is empty on this branch, and
      no override in that file filters on any touched test — the only
      `claudine-cli` L1 entry is the `claudine-cli-ci-l1` test-group
      assignment.)
- [x] Local robustness gate (AC 6): run each touched timeout test 10× in a
      row under full `just test-cli` load with zero failures. (Proof of
      budgets is CI duration, Phase 7 — never local wall clock.)
      (10 consecutive full `just test-cli` runs, 2396 passed / 10 skipped
      every time, zero failures. Worst-of-10 for each touched test:
      `watchdog_stream_idle` 1.724 s · `watchdog_subagent_hang` 1.716 s ·
      `inline_compose_non_harness_respects_cli_step_timeout` 1.681 s ·
      `compose_non_harness_respects_cli_timeout` 1.457 s ·
      `watchdog_wall_clock` 1.449 s · `post_fanout` 1.305 s ·
      `sequence_per_step_step_timeout_override` 1.285 s · both
      `opencode_stderr_*_forces_early_termination` ≤ 0.397 s. Every one sits
      inside budget + tick + 1 s.)
- [x] Non-vacuity of the shortened budgets: neuter one and confirm red.
      (`sequence_schema.rs` step budget `0.5s` → `5s` ⇒ elapsed 5.588 s trips
      the tightened 4 s bound; `watchdog_subagent_hang`'s `1s` → `60s` ⇒ the
      run never terminates and the process guard trips. Both reverted.)

Validation (Phase 5 checkpoint):

- [x] `just test-cli` green; the six watchdog tests plus the sequence and
      opencode timeout tests show visibly lower local durations.
      (2396 passed / 10 skipped, 15.3 s. Before → after, same host:
      `watchdog_subagent_hang` 3.333 → 1.490 s ·
      `inline_compose_non_harness` 2.323 → 1.486 s ·
      `watchdog_stream_idle` 2.320 → 1.491 s ·
      `compose_non_harness_respects_cli_timeout` 2.313 → 1.328 s ·
      `watchdog_wall_clock` 2.298 → 1.245 s ·
      `sequence_per_step_step_timeout_override` 1.852 → 1.209 s ·
      `post_fanout` 1.219 → 1.175 s. Serial sum for
      `wrap_watchdog_timeout` 13.8 → 8.2 s.)
- [x] Every changed budget line has its margin comment.
      (Per-site comments name the fixture-specific margin; the shared
      1s / 0.2s / 0.5s derivation is a new `## Budget sizing` module-doc
      section in `wrap_watchdog_timeout.rs`.)

## Phase 6 — The shipped feature-review contract test (RB 5)

Depends on: Phase 1 (builder) and Phase 3B (shipped_prompt_contract
migration). Parallelizable with Phases 4–5.

- [x] Rework `feature_review_cli_preserves_numeric_iteration_and_dependent_paths`
      to copy the repo-root `prompts/` corpus (45 Markdown files,
      byte-for-byte, preserving relative layout) into the fixture workspace,
      so the `::file ../_senior-reviewer.md` reference and `@{{spec}}` file
      references resolve inside an isolated repository and
      `derive_request_context_for_source` no longer anchors on the monorepo.
      Keep the absolute `spec=` path (out-of-scope item 6).
      (`copy_shipped_prompts` copies the 45 shipped `.md` files into
      `<fixture cwd>/prompts`, the fixture `cwd` is `git init`-ed so the
      corpus sits in an isolated repository, and the feature stays under the
      fixture HOME so `dirname(spec)` keeps its `~/` projection arm. A new
      assertion pins the inlined `_senior-reviewer.md` opening line, so the
      relative `::file ../` span is proven to resolve against the copy.)
- [x] Local attribution proof: run the test's command with `--perf` and
      confirm repository discovery no longer appears in the profile; record
      the output (labeled local) for the inventory.
      (Local, macOS, same fixture shape, 3 runs each. **Copied corpus:**
      total 285–344 ms, prep phase 80.5/80.6/84.5 ms, frontmatter load
      2.6–2.8 ms, `Git discoveries 1, topology probes 1, topology reuses 1`
      — the launch capture's own probe of the fixture repo, reused by the
      source derivation. **Monorepo prompt, everything else identical:**
      total 554 ms, prep phase 339.3/341.8/346.3 ms, frontmatter load
      261.7/262.8/268.5 ms flagged `▇ HOT`, `Git discoveries 2, topology
      probes 2, topology reuses 0` — the second probe being the rusty-biscuit
      topology walk. Both compose byte-identical output, 78 lines with
      `> - Review Iteration: #3`.)
- [x] AC 3 isolation proof (any one migrated dry-run test, local): `--perf`
      reports launch discovery under 5 ms and no repository system prompt;
      and running it from a checkout whose root `system-prompt.md` has been
      modified produces identical output. Record both observations.
      (Subject: `wrapper_dry_run_prints_command_and_exits_zero`'s command
      — `codex --dry-run -- --version` from the fixture. `launch discovery`
      **245–247 µs** of a 5.7 ms total; `topology probes 0`; the dry-run
      report carries no `System prompt:` section at all. Appending a marker
      line to the checkout's root `system-prompt.md` and re-running produced
      output identical except the per-process `CLAUDINE_PID` and
      `CLAUDINE_SESSION_ID`; the checkout file was restored byte-for-byte
      — md5 `3cbc8787…` before and after, `git status` clean.)

Validation (Phase 6 checkpoint):

- [x] `just test-cli` green; the feature-review test's local duration drops
      to the same shape as neighboring isolated tests.
      (2397 passed / 10 skipped, 14.2 s. The feature-review test is 0.37 s
      against the copy and 0.69 s when temporarily pointed back at the
      monorepo prompt, on an otherwise identical run; its neighbors —
      `compose_header_first`, `inline_compose_hash`, `wrap_compose_exec` —
      sit at 0.20–0.32 s.)
- [x] The `--perf` captures are saved for Phase 7's inventory record.
      (Recorded in the two bullets above; the reproduction scripts are
      `/tmp/fr_perf.sh` and `/tmp/ac3_proof.sh` on this host.)

Non-vacuity of the new coverage (neuter → red → restore):

- [x] `copy_shipped_prompts` dropping the first corpus file ⇒
      `copied_prompt_corpus_matches_the_shipped_tree` fails on the layout
      assertion while the E2E still passes — which is exactly why the corpus
      test exists. Dropping `_senior-reviewer.md` instead reddens **both**:
      the E2E dies with `File not found: ../_senior-reviewer.md`, proving the
      relative span resolves inside the copy rather than in the checkout.
      Both edits reverted.

## Phase 7 — Verification of record, docs, and CI proof (RB 6, RB 7; AC 1–10)

Depends on: all previous phases. Longest phase (CI wall clock); start the CI
leg as soon as the tree is otherwise complete.

- [x] Guard non-vacuity, full demonstration (AC 2): add a raw
      `cargo_bin("claudine")` spawn to one migrated file → guard fails;
      remove a live allowlist entry → guard fails; revert both; show the
      transcripts in the PR description.
      (Three arms, each reverted and re-verified green. **(a) Raw spawn in a
      migrated file:** appended an `assert_cmd::Command::cargo_bin("claudine")`
      probe to `wrap_basics.rs` ⇒ `Unlisted sites: wrap_basics.rs:828
      cargo_bin` — exactly one site, which is also the proof the migrated file
      carries none of its own. **(b) Deleted a live allowlist entry:** removed
      the `compose_cli.rs` entry ⇒ `Unlisted sites: compose_cli.rs:52 / :115 /
      :211 / :315 cargo_bin`, and the burn-down roll-up dropped to
      `34 file(s), 165 site(s)`. **(c) Stale-entry arm:** rewrote
      `protect_cli.rs`'s only site as `cargo_bin(["cl", "audine"].concat())` so
      the detector cannot see it while its entry stays ⇒ `Stale
      SPAWN_ALLOWLIST entries name files with no raw spawn site left … Stale
      entries: protect_cli.rs`. After all three reverts the guard passes and
      `spawn_site_guard.rs` / `wrap_basics.rs` / `protect_cli.rs` hash back to
      their pre-demonstration bytes.)
- [x] Docs: update the `rust-testing` skill and the Claudine skill's testing
      notes to point at the builder as the L1 spawn contract and at the
      guard; refresh skill frontmatter hashes with `md hash <file>` (RB 7,
      second half; AC 10).
      (New `## Spawning the Binary Under Test (L1)` section in
      `.claude/skills/rust-testing/SKILL.md` — the cost/correctness pair, the
      fixture/builder/guard table, the default-`PATH` rule and the three
      escapes, and the stale-entry allow-list mechanic — plus an **L1 spawn
      contract** paragraph in `.claude/skills/claudine/SKILL.md` naming
      `CliProcessFixture`, the three escapes, and `spawn_site_guard.rs`.
      `md hash` re-stamped `rust-testing` to
      `1acc7c1c76b11142-e852f9f6596146b8` with `last_updated: 2026-09-06`; the
      Claudine skill carries no `hash:` property, so nothing to re-stamp
      there.)
- [x] Revisit `common/mod.rs` module docs for drift after migration learnings;
      fix or delete drifted inline comments in touched files (monorepo comment
      discipline).
      (`common/mod.rs`: the module docs now name `spawn_site_guard.rs` and both
      forms it governs; `claudine_bin` says it is a path carrying none of the
      contract, and why L2/L3 want exactly that; `augmented_path` gained the
      docs it never had, naming `host_path` as the L1 route in. One drifted
      inline comment in `wrap_basics.rs` claimed the call site isolated `HOME`
      — the builder does that now — so it was rewritten to explain what the
      call site still decides, keeping the leaked-handle reason. The nine
      `#[cfg(unix)]` "requires Unix HOME isolation" notes in `mcp_cli.rs` were
      read and left: they justify a platform gate this fix did not touch.)
- [x] Local canonical verification from the `claudine` package area:
      `just test-cli`, `just lint`, `just ci-local`, and `just test-l2` all
      green (AC 9). No `cargo test` anywhere in the workflow.
      (`just test-cli` 2397 passed / 10 skipped, 14.1 s. `just lint` clean
      across all five crates. `just test-l2` 230 passed / 2411 skipped for
      `claudine-cli` (53.6 s) plus 3 passed for `claudine-gen`. `just ci-local`
      is a **root** recipe — the area justfile does not import it — so it ran
      as `just ci-local claudine-cli` from the repo root: lint ✅ and L1 ✅,
      2402 passed / 244 skipped under the CI feature set, 3 m 4 s. The count
      difference is the CI-only `daemon-tests`/`terminal-tests` targets, not
      dropped tests.)
- [ ] **BLOCKED (needs a push).** Open/land the change and collect **three
      consecutive green runs** on `ubuntu-latest`, `macos-latest`,
      `windows-latest`, and `wsl2-ubuntu` (PR runs + `main` runs;
      `NEXTEST_PROFILE=ci` JUnit artifacts `junit-*` are the record).
      (Nothing on this branch is committed — committing and pushing are
      separate, human-driven steps — so no post-change CI run exists. This is
      the one Phase 7 item that cannot be produced from this host.)
- [x] From those JUnit artifacts, compute per environment: migrated tests
      ≥ 5 s and ≥ 2 s (non-timeout), slowest migrated non-timeout test,
      per-test timeout-test durations vs budget + tick + 1 s (native) / + 2 s
      (WSL2), and serial sums for the 19 and 10 binary groups. Compare
      against the Outcome table.
      (The computation is implemented as
      [junit-metrics.ts](junit-metrics.ts) — binary lists, the nine
      timeout-shaped tests with their `budget`/`tick` floors, and environment
      discovery — and **validated against the baseline**: run `33440897014`'s
      four artifacts are still live, and the script reproduces the spec's
      Problem-section table to the decimal (237.9 / 158.7 s Ubuntu, 178.2 /
      118.4 s macOS, 4.9 / 63.5 s Windows, 1207.3 / 846.7 s WSL2; 6/3/4/89
      tests ≥ 5 s; 14.3 / 9.6 / 8.8 / 77.1 s slowest). The post-change columns
      wait on the blocked item above; the script also reads a local
      `NEXTEST_PROFILE=ci` report, recorded as attribution.)
- [x] Write the "2026-09 follow-up" section of [inventory.md](inventory.md):
      baseline run `33440897014` versus the first three green runs, per
      environment, with the table above plus the local `--perf` attribution
      captures. A missed target is recorded as a miss with its cause — never
      adjusted to fit (RB 6; AC 4).
      (Section written with the reproduction recipe, the computed baseline
      table, an explicitly empty post-change table naming what blocks it, and
      the local attribution: one `NEXTEST_PROFILE=ci` run on this host gives
      0 tests ≥ 5 s, 0 ≥ 2 s non-timeout, 0.6 s slowest, 28.5 s / 7.0 s serial
      sums, and all nine timeout tests inside `budget + tick + 1 s` — against
      a `macos-latest` baseline of 178.2 s / 118.4 s. Plus the `--perf`
      captures from Phases 3 and 6.)
- [x] Final acceptance sweep: walk AC 1–10 one by one and record
      pass/miss evidence in the PR description; on any miss, report it and
      its cause rather than re-opening scope silently.
      (Table below. AC 4 is **not met and not missed** — it is unstarted,
      because it needs CI runs of a branch nobody has pushed yet; AC 6 and
      AC 8 are partial for the same reason. Re-verified 2026-09-06, which
      downgraded AC 8 from pass, named the AC 5 snapshot deviation, and
      recorded an unimplemented RB 1 bullet as an open spec gap below.)

### Acceptance sweep

| AC | Verdict | Evidence |
|---|---|---|
| 1 — builder is the sole spawn path | **pass** | Guard green; its roll-up reads `169 raw sites in 35 allow-listed files (of 169 sites scanned)`, all `outside this fix's scope`. A name-by-name check of the 29 binaries plus `ctx_launch_anchor.rs` and `propagated_context_fixtures.rs` finds none of them in `SPAWN_ALLOWLIST`. |
| 2 — guard is non-vacuous | **pass** | Three demonstrations, all reverted: raw spawn added to `wrap_basics.rs` ⇒ one unlisted site; `compose_cli.rs` entry deleted ⇒ four unlisted sites; `protect_cli.rs`'s site hidden from the detector ⇒ stale entry. Transcripts in the Phase 7 task note. |
| 3 — isolation is proven | **pass** | Phase 6: `launch discovery` 245–247 µs of 5.7 ms, `topology probes 0`, no `System prompt:` section; identical output against a checkout whose root `system-prompt.md` was modified. |
| 4 — CI targets met on three consecutive green runs | **unstarted (blocked)** | Requires a push; nothing on this branch is committed. Measurement tooling is built and validated against the baseline artifacts, so the four post-change rows are a `gh run download` away. |
| 5 — assertions unchanged | **pass (with one recorded deviation)** | Phase 3/4 spot-review: previously-isolated tests diff as environment-setup deletions. Two assertions and three comments needed a non-environment change and are named in their phase reports. **Deviation:** `snapshots/wrap_basics__wrapper_reports_removed_sensitive_env_names.snap` was rewritten (2 lines replacing 4) because the call site moved `TERM_WIDTH` from `80` to `200`, un-wrapping two blocks — a wrapped temp path cannot be redacted to `<workspace>`, so an 80-column snapshot would differ between macOS and Linux. Once the builder owns the render width (see the open spec gap below), this call site should drop the manual `TERM_WIDTH` and the snapshot be regenerated once more. |
| 6 — timeout budgets justified and proven | **partial** | Local half done: every changed budget carries its margin comment, `wrap_watchdog_timeout.rs` gained a `## Budget sizing` module-doc section, and 10 consecutive full `just test-cli` runs passed with every touched test inside budget + tick + 1 s. The CI half rides on AC 4. |
| 7 — no slow-timeout overrides added | **pass** | `git diff main -- .config/nextest.toml` is empty and the file is unmodified in `git status`. |
| 8 — cross-platform | **partial (rides on AC 4)** | The *guard* half passes: `cli_process_fixture.rs` proves the contract through a recording provider stub named `claude` — the fixture stub wins resolution over any host install, no Homebrew/npm/cargo/`.local`/`Program Files` prefix and no non-minimal host `PATH` entry reaches the child, `XDG_CONFIG_HOME`/`HOMEDRIVE`/`HOMEPATH` are removed against a live parent-environment control, and both ambient-context rejections are covered. No terminal or browser window is involved. The *exercised-on-four-environments* half has **not** happened: this is a macOS-only run, and the `#[cfg(windows)]` arms (`minimal_system_path()`, the `.cmd` recording stub, the `%SystemRoot%\System32` assumptions) have never been compiled — they first compile on the `windows-latest` CI leg. Downgraded from **pass** on 2026-09-06 because AC 8's text asks for all four CI environments. |
| 9 — verification through canonical recipes | **pass (one host flake)** | Re-verified 2026-09-06: `just test-cli` 2397 passed / 10 skipped (14.0 s), `just lint` clean across all five crates, `just ci-local claudine-cli` (lint ✅, L1 2402/244). No `cargo test` anywhere. `just test-l2` is 229/230: `level2_initialize_proxy_block_auto_detects_osc8_in_wezterm` times out at 32 s in the *full* 230-test run (reproduced 3×) but passes in 2.1 s alone and alongside the other three WezTerm tests. Cause is a **host** artifact, not this branch: the captured pane shows an interactive `Atuin AI is not yet configured` prompt swallowing the shell input, so the `claudine_rc:` exit marker never arrives. The file is untouched by this fix and the `common/` helpers it imports still resolve. |
| 10 — docs and skills in the same change | **pass** | `rust-testing` SKILL.md gained `## Spawning the Binary Under Test (L1)` and was re-hashed; the Claudine skill gained an **L1 spawn contract** paragraph; `common/mod.rs` module docs describe the contract and name the opt-outs. |

### Open spec gap — Required behavior 1's environment-inheritance contract

Surfaced by [review-2.md](review-2.md) finding 1 and **independently reproduced
on this branch during the Phase 7 sweep**. RB 1's fourth bullet asks the builder
to scrub the inherited `CLAUDINE_*` namespace and the `GIT_DIR`/`GIT_WORK_TREE`/
`GIT_INDEX_FILE`/`GIT_COMMON_DIR`/`GIT_OBJECT_DIRECTORY` family, and to pin the
rendering inputs `TERM_WIDTH`/`COLUMNS`/`FORCE_COLOR`. None of it is in
`ClaudineCommandBuilder::build`, and the bullet never reached this plan — a grep
for `GIT_DIR`, `TERM_WIDTH`, `COLUMNS`, `FORCE_COLOR`, `env_remove` across the
phases above returns zero hits, so it was lost between spec and plan rather than
attempted and deferred.

Reproduced from the `claudine` package area, both turning **migrated** tests red:

```bash
CLAUDINE_STEP_TIMEOUT=0.3s CLAUDINE_TIMEOUT=0.3s just test-cli watchdog
#   3 failed: watchdog_subagent_hang_terminates_and_names_stuck_ids,
#             watchdog_stream_idle_timeout_after_tool_call_hang,
#             watchdog_opencode_post_fanout_silence_does_not_kill_prematurely
FORCE_COLOR=1 just test-cli non_tty_withholds_yaml
#   1 failed: inline_compose_sequence_mismatch::non_tty_withholds_yaml_but_keeps_guidance
```

`FORCE_COLOR` is *not* neutralized by the builder's `NO_COLOR=1`:
`cli/src/commands/compose/mod.rs` and `cli/src/commands/sequence.rs` gate the
frontmatter YAML appendix on `env::var_os("FORCE_COLOR")` directly. The
`CLAUDINE_*` arm re-parameterizes exactly the six timeout tests Phase 5 tightened,
so the failure mode is "your machine says the budgets are wrong".

No acceptance criterion tests this bullet directly, which is why the sweep above
can still stand — but the fix does not fully implement RB 1, and closing it is
prerequisite work for the push, not follow-up. It is deliberately **not**
implemented here: it is Phase 1 scope, and reopening the builder inside the
closing verification phase would silently widen this phase. See review-2's
"Recommended order of work" step 1, including the landmine that
`cli_process_fixture.rs`'s `CLAUDINE_PROBE_CONTROL` control must be renamed
before a `CLAUDINE_*` scrub can land.

Validation (Phase 7 checkpoint — the release gate):

- [x] All ten acceptance criteria from spec.md have recorded evidence.
      (Eight with pass evidence, AC 4 unstarted with a recorded reason, AC 6
      and AC 8 half-recorded because both ride on the same blocked CI runs.)
- [ ] **BLOCKED (needs a push).** Outcome-table targets met on three
      consecutive green runs per environment, or each miss is documented with
      cause.
- [x] Inventory updated; skills updated and re-hashed; guard green with an
      allowlist naming no migrated file.

---

## Parallelism map

| Work | May run concurrently with |
|---|---|
| Phase 3 Batches A–D | Each other, and all of Phase 4 |
| Phase 4 files | Each other, and Phase 3 |
| Phase 5 | Phase 4 (other files), Phase 6 |
| Phase 6 | Phase 4, Phase 5 |
| Phase 7 CI leg | Nothing — closing gate |

## Dependency order (summary)

```
Phase 1 (builder)
   └─ Phase 2 (guard + seeded allowlist)
        ├─ Phase 3 batches A–D ─┐
        ├─ Phase 4 ─────────────┼─ Phase 7 (verify, docs, CI proof)
        ├─ Phase 5 (needs 3B/3C/3D files) ─┘
        └─ Phase 6 (needs 3B)
```

## Out of scope (guard rails)

Anything the spec's "Out of scope" section names: launch-discovery cost /
placement in production code, root `system-prompt.md` composition, watchdog
cadence defaults (coordinate with the 2026-08-31 spec only), rendezvous
reporting default, the stale-identity/tautological-assertion test-maintenance
items, the relative `spec=` inconsistency, and all non-spawn-shaped slow
tests. No nextest `slow-timeout` overrides. No behavior changes to what tests
assert.
