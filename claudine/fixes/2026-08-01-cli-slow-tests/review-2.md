---
$schema: feature-review.yaml
ready: false
agent: claude/default
created: 2026-09-06T16:55:14-07:00
spec: 2026-08-01-cli-slow-tests/spec.md
implemented: true
description: A **fix** review of `2026-08-01-cli-slow-tests/spec.md`
fix: 2026-08-01-cli-slow-tests/review-2.md
previous: 2026-08-01-cli-slow-tests/review-1.md
next: 2026-08-01-cli-slow-tests/review-3.md
---

# Review 2 — `2026-08-01-cli-slow-tests`

## Verdict

**Not production ready.** Two things block it, one of them substantive.

1. **A required behavior was dropped at planning time and never implemented.**
   Required behavior 1's environment-inheritance contract — scrub the inherited
   `CLAUDINE_*` namespace and the `GIT_DIR`/`GIT_WORK_TREE`/`GIT_INDEX_FILE`/
   `GIT_COMMON_DIR`/`GIT_OBJECT_DIRECTORY` family, pin `TERM_WIDTH`/`COLUMNS`/
   `FORCE_COLOR` — is absent from `ClaudineCommandBuilder::build`. This is not a
   theoretical gap: I reproduced all three leaks on this branch, and each one
   turns *migrated* tests red (finding 1). The spec's Problem section §5 exists
   because one of these families already caused a real incident on 2026-08-31.
2. **The Outcome table has no evidence.** Nothing is committed or pushed, so
   acceptance criterion 4 is unstarted and criterion 6's CI half rides on it.
   Every number the fix claims comes from one macOS developer host, which the
   spec explicitly forbids as proof ("Local runs are for attributing cost, never
   for proving a target").

Item 2 is honestly recorded in both `plan.md` and `inventory.md` — the
implementer did not dress a local run up as a CI result, and the measurement
script is written and validated against the live baseline artifacts. That is the
right posture, and it means item 2 closes by pushing rather than by more work.
Item 1 needs code.

Everything else is in good shape. The builder, the guard, the migration of all
29 binaries, the timeout budgets and the shipped-prompt corpus copy are careful,
well-commented work, and the escape call sites carry the comments the spec asks
for without exception. `just test-cli` is green here (2397 passed / 10 skipped,
17.5 s) and `just lint` is clean across all four claudine crates.

## What was verified for this review

| Check | Result |
|---|---|
| `just test-cli` (claudine area) | green — 2397 passed, 10 skipped, 17.5 s |
| `just lint` (claudine area) | green — claudine, claudine-contract, claudine-cli, claudine-gen |
| Guard allow-list vs. the 29 binaries | none of the 29, nor `ctx_launch_anchor.rs` / `propagated_context_fixtures.rs`, appear in `SPAWN_ALLOWLIST` |
| Residual `augmented_path` call sites in L1 | 0 in the 29; all remaining callers are allow-listed out-of-scope files or `level2_*` |
| Escape call-site comments | every `fake_only_path` / `host_path` / `ambient_context` / `inherit_no_env` site is commented |
| Skill hash | `md hash .claude/skills/rust-testing/SKILL.md` matches the stamped `1acc7c1c76b11142-e852f9f6596146b8` |
| Env-leak probes | three families still reach the child — see finding 1 |

`review-1.md` is not on disk and not in git history anywhere in this repo
(`git log --all -- 'claudine/fixes/2026-08-01-cli-slow-tests/review-1.md'`
returns nothing), so its `next`/`implemented` frontmatter could not be updated.
The spec's `review_iterations` has been advanced to `2`.

## Test rigor by requirement

Test count is not the issue here — the coverage that exists is real and mostly
non-vacuous. The issue is that three requirements are verified one level below
what they demand. For this fix the levels that matter are **L1 (in-process /
child-process probe)**, **CI-evidence** (the spec's own reference environment:
four environments × three green runs, JUnit artifacts), and **cross-platform
execution** (the Windows and WSL2 legs actually compiling and running the code).

| Requirement | Level demanded | Level present | Verdict |
|---|---|---|---|
| RB 1 — hermetic builder: CWD, HOME family, `PATH` | L1 | L1 — `cli_process_fixture.rs` asserts on what a recording provider stub actually received, not on builder internals | **met**, and the probe design is the right one |
| RB 1 — environment inheritance (`CLAUDINE_*`, `GIT_*`, render inputs) | L1 | **none** — unimplemented and untested | **gap (critical)** — finding 1 |
| RB 2 — all 29 binaries migrated | L1 structural | L1 — `spawn_site_guard.rs` | **met** for the two forms it detects; a third live form is invisible — finding 4 |
| RB 3 — guard non-vacuity | L1 unit + manual demonstration | both, three arms, each reverted and re-verified | **met** |
| RB 3 — roll-up observable from a CI log | CI log output | `eprintln!` from a passing test; nextest's `success-output` defaults to `never` | **gap (medium-low)** — finding 6 |
| RB 4 — timeout budgets | **CI**, explicitly ("Budgets are proven on CI, not locally"; each test on four environments × three runs) | 10 consecutive local `just test-cli` runs on one macOS host | **level mismatch (high)** — finding 2 |
| RB 5 — shipped feature-review contract | L1 + local `--perf` attribution | both, plus a corpus-fidelity test with a recorded neuter→red→restore | **met** |
| RB 6 — measurement of record | CI JUnit artifacts, three green runs | tooling written and validated against the baseline run; post-change rows empty | **unstarted (high)** — finding 2 |
| AC 8 — cross-platform | execution on all four CI environments | macOS only; the `#[cfg(windows)]` arms have never been compiled | **level mismatch (high)** — finding 3 |
| RB 7 — docs and skills | n/a | both skills updated in-change, `rust-testing` re-hashed | **met** |

## Findings

### 1. (critical) The environment-inheritance contract is unimplemented, and all three families demonstrably leak

`ClaudineCommandBuilder::build` (`claudine/cli/tests/common/mod.rs:338`) sets
`HOME`, `USERPROFILE`, `APPDATA`, `LOCALAPPDATA`, `PATH`,
`CLAUDINE_RENDEZVOUS_REPORT`, `NO_COLOR`, and removes `HOMEDRIVE`, `HOMEPATH`,
`XDG_CONFIG_HOME`. Required behavior 1's fourth bullet asks for more than that,
and none of it is there. `plan.md` never carried the bullet either — a grep for
`GIT_DIR`, `TERM_WIDTH`, `COLUMNS`, `FORCE_COLOR`, `env_remove` across the plan
returns zero hits — so this was lost between spec and plan rather than attempted
and deferred.

Reproduced on this branch, each from the `claudine` package area:

```bash
CLAUDINE_TIMEOUT=0.3s CLAUDINE_STEP_TIMEOUT=0.3s just test-cli watchdog
#   3 failed: watchdog_stream_idle_timeout_after_tool_call_hang,
#             watchdog_subagent_hang_terminates_and_names_stuck_ids,
#             watchdog_opencode_post_fanout_silence_does_not_kill_prematurely

GIT_DIR=/tmp/probe/.git GIT_WORK_TREE=/tmp/probe just test-cli
#   3 failed, including
#   cli_process_fixture::ambient_context_escape_pins_the_cwd_to_a_test_built_repository

FORCE_COLOR=1 just test-cli non_tty_withholds_yaml
#   1 failed: inline_compose_sequence_mismatch::non_tty_withholds_yaml_but_keeps_guidance
```

Three observations sharpen this:

- The `GIT_*` arm defeats **the hermeticity proof itself**. The one test whose
  job is to show that the ambient-context escape can only ever anchor on a
  repository the test built is the test an inherited `GIT_DIR` breaks.
- The `FORCE_COLOR` arm is *not* neutralized by the builder's `NO_COLOR=1`.
  `cli/src/log.rs` does short-circuit on `NO_COLOR`, but
  `cli/src/commands/compose/mod.rs:453` and `cli/src/commands/sequence.rs:234`
  gate the frontmatter YAML appendix on `std::env::var_os("FORCE_COLOR").is_some()`
  directly. A reviewer's or CI shell's exported `FORCE_COLOR` therefore changes
  what a migrated test sees.
- `wrap_basics.rs` had to hand-pin `TERM_WIDTH=200` at the call site (and rewrite
  its snapshot to match) precisely because the builder does not own that input.
  That is the requirement asserting itself through a workaround.

The `CLAUDINE_*` arm is the one with the sharpest user-facing consequence: the
tests it re-parameterizes are the six timeout tests this fix just tightened to
sub-second budgets, so the failure mode is "your machine says the budgets are
wrong" — the most expensive kind of false signal to debug.

**One landmine when fixing this.** `cli_process_fixture.rs:199` uses
`CLAUDINE_PROBE_CONTROL` as its "the parent's environment really does reach the
child" control and asserts at line 206 that the child received it. That
assertion currently *encodes* the absence of a `CLAUDINE_*` scrub. Rename the
control to a non-`CLAUDINE_` name (`FIXTURE_PROBE_CONTROL`) before adding the
scrub, and then add the three positive tests the scrub deserves: an inherited
`CLAUDINE_STEP_TIMEOUT` does not reach the child, an inherited `GIT_DIR` does
not, and a call site that sets `CLAUDINE_STEP_TIMEOUT` *after* taking the builder
still wins (the spec's "removal is per key at build time" rule).

### 2. (high) Acceptance criterion 4 is unstarted; the Outcome table is unverified

No post-change JUnit artifact exists because the branch is uncommitted. The
spec's reference environment is CI and it is unambiguous that local runs never
prove a target, so at present the fix has:

- zero of the required 12 data points (4 environments × 3 consecutive green runs);
- one local macOS `NEXTEST_PROFILE=ci` run standing in for all of them (0 tests
  ≥ 5 s, 0 non-timeout ≥ 2 s, 28.5 s / 7.0 s serial sums);
- a validated measurement script (`junit-metrics.ts`) that reproduces the
  baseline table to the decimal, which is genuinely good preparation.

AC 6 inherits the same gap: every shortened budget carries its margin comment and
survived 10 consecutive local suite runs, but "each timeout test's CI duration
sits within budget + tick + 1 s (native) / + 2 s (WSL2) with zero failures across
three runs" has not been observed once. The WSL2 leg is the one that matters —
the baseline shows the same test shapes running 20–50× slower there, and the
`0.5s` step budget in `sequence_per_step_step_timeout_override` is the value the
plan itself flags as having the thinnest margin under contention.

This is a "push it and read the artifacts" item, not a design problem. It is
listed as high because "ready for production" cannot be asserted over it.

### 3. (high) The Windows and WSL2 code paths have never been compiled

`minimal_system_path()`'s `#[cfg(windows)]` arm, the `.cmd` recording stub in
`cli_process_fixture.rs`, and the `%SystemRoot%\System32` assumptions throughout
are first compiled on the `windows-latest` CI leg. The plan states this caveat
plainly (Phase 1, "the workspace cannot cross-compile to Windows"), which is
fair — but `plan.md`'s acceptance sweep then marks AC 8 **pass** on the strength
of a macOS run. That is one level short of what AC 8 asks ("exercised on all four
CI environments").

Two concrete Windows risks worth pre-empting before the push rather than
discovering from a red leg:

- The spec is explicit that `System32` resolves **none** of `sh`, `cat`, `sleep`,
  or `git`. 17 of the 161 inventoried tests do compile on Windows; any of them
  whose fixture shells out by bare name will now fail there where it previously
  inherited the runner's full `PATH`. A pre-push read of the Windows-compiled
  subset for bare-name utility use would cost minutes and save a CI round trip.
- `PATHEXT` survives the default (correct), but `inherit_no_env()` calls
  `env_clear()` and takes `PATHEXT`, `COMSPEC`, and `SystemRoot` with it, which
  is exactly what `.cmd` stub resolution needs. The module docs warn about this
  and the only current call site is `#[cfg(unix)]`, so it is latent rather than
  broken — but the next Windows-compiled caller of `inherit_no_env` will hit it.
  Consider re-adding `PATHEXT`/`SystemRoot`/`COMSPEC` inside `build()` when
  `inherit_env == false` on Windows, so the tightening knob stays usable.

### 4. (medium) A third live spawn form is invisible to the guard

`claudine/cli/tests/wrap_ctrl_c_windows.rs:130` obtains the binary with
`biscuit_test_harness::bin_exe!("claudine")` and spawns it with
`std::process::Command::new`, giving the child a **full host `PATH`** (built
inline at lines 124–128) and an untouched `CLAUDINE_*` / `GIT_*` environment.

The file's own module docs argue at length that it is an ordinary **Level 1**
test, not L2 or L3 — so it is squarely inside the guard's contract. It carries no
`level2_`/`level3_`/`real_` prefix, so `excluded()` does not skip it, and it has
no `SPAWN_ALLOWLIST` entry. The guard nevertheless reports zero sites for it,
because `spawn_sites()` only knows `cargo_bin` and `claudine_bin`. Because the
file is `#[cfg(windows)]`, no macOS or Linux run will ever surface this.

Consequences: the burn-down census under-reports, and `bin_exe!` is now the
frictionless way to add a non-hermetic L1 spawn — the exact regrowth the guard
exists to prevent.

Fix: add `FORM_BIN_EXE` to the detector, keyed on the `bin_exe` identifier with
the same `names_claudine` literal check (the macro's argument is a plain
`"claudine"` literal, so the existing helper works unchanged), then either
migrate `wrap_ctrl_c_windows.rs` onto the builder or give it an allow-list entry
naming the reason (it needs `CREATE_NEW_PROCESS_GROUP`, which `assert_cmd` does
not expose). Add a detector unit test for the new form alongside the existing
two, and check `sequence_ctrl_c_windows.rs` for the same shape.

### 5. (medium) Isolation can still be defeated *after* `command()`, and the guard is blind to it

`build()` hands back a bare `assert_cmd::Command`. Nothing prevents a future call
site from writing `.current_dir(repository_root())` or
`.env("PATH", augmented_path(&dir))` on it, which reinstates both leaks the fix
removed — and the spawn form remains perfectly legitimate, so the guard says
nothing. I checked all 29 migrated binaries and there is **no live violation**
today; this is about whether "hermetic by construction" survives the next six
months.

Two options, in increasing strength:

- Cheap: extend the source scan to flag `.current_dir(`, `.env("PATH"`, and
  `env_clear()` in governed files, on the same allow-list mechanics. The escapes
  already exist for every legitimate need, so the false-positive rate should be
  near zero.
- Stronger: return a small wrapper type exposing `arg`/`args`/`env`/`assert`/
  `output`/`timeout` and nothing else, funnelling CWD and `PATH` through builder
  methods. This costs a mechanical pass over the 29 files and makes the contract
  a type rather than a convention.

### 6. (medium-low) The burn-down roll-up never reaches a CI log

Required behavior 3 asks that the roll-up be "observable from a CI log without
re-running a census". It is emitted with `eprintln!` from
`l1_tests_spawn_claudine_through_the_fixture_builder`, which passes — and
`.config/nextest.toml` sets no `success-output`, whose nextest default is
`never`. The roll-up is therefore visible only on the runs where the guard fails,
which is precisely when nobody needs the census.

Options: write the roll-up to `$BISCUIT_JUNIT_STAGE_DIR` (or
`target/nextest/ci-reports`) as a small JSON/text artifact the way
`backend-executions.jsonl` is written, or add a narrowly-scoped
`success-output = "final"` override filtered to this one test. The former fits
the repo's existing evidence conventions better.

### 7. (low) An AC 5 deviation is not recorded

`claudine/cli/tests/snapshots/wrap_basics__wrapper_reports_removed_sensitive_env_names.snap`
changed: two blocks that used to word-wrap at 80 columns are now single lines,
because the call site moved `TERM_WIDTH` from `80` to `200`. The reason is sound
and the test carries a good four-line comment explaining it (a wrapped temp path
cannot be redacted to `<workspace>`, so the snapshot would differ between macOS
and Linux). But `plan.md`'s acceptance sweep marks AC 5 pass with "two assertions
and three comments needed a non-environment change" and does not name a snapshot
rewrite. Name it, so the record matches the diff. (Once finding 1 is fixed and
the builder pins the width itself, this call site should drop the manual
`TERM_WIDTH` and the snapshot should be regenerated once more.)

### 8. (low) `IN_SCOPE` is dead vocabulary kept alive by its own unit test

`spawn_site_guard.rs:56–62` keeps a constant no production path uses, with a
six-line doc comment explaining that the only remaining consumer is
`reconciliation_reports_unlisted_sites_stale_and_unexplained_entries`, which
"needs *some* reason string". That test can use a string literal. The constant
plus its justification is exactly the kind of comment the monorepo's comment
rules ask to delete: a future burn-down will invent its own vocabulary anyway.

### 9. (low) The fixture root follows `TMPDIR`, with nothing checking where that lands

`TestWorkspace::named` builds under `std::env::temp_dir()`. If `TMPDIR` ever
points inside the checkout — a developer's scratch setting, or a CI leg that
redirects temp to the workspace to keep it on a fast disk — every fixture
workspace lands back inside the repository, repository discovery starts walking
the 35-member workspace again, and the entire fix silently reverts with a green
suite. A one-line assertion in `CliProcessFixture::named` that the workspace root
is not under the repository root would close it, and it composes naturally with
the containment check `ambient_context` already performs.

## What is done well

Worth recording, because these are the parts that should not be re-litigated:

- **The probe design in `cli_process_fixture.rs`.** Asserting on what a recording
  provider stub actually received, rather than on the builder's fields, is the
  right way to test this and is what makes the `PATH`-isolation guard (AC 8)
  meaningful rather than tautological. Naming the stub `claude` — a provider a
  developer machine plausibly has installed — so that "the stub ran" is itself
  the proof that no host install won selection, is a nice touch.
- **The three named escapes and their discipline.** Every one of the 20-odd
  escape call sites across `wrap_compose_agent.rs`, `argv_normalization.rs`,
  `mcp_cli.rs`, `wrap_opencode.rs`, `ctx_launch_anchor.rs`,
  `propagated_context_fixtures.rs`, and `wrap_basics.rs` carries a comment naming
  the tool or the proof it depends on. That is unusual follow-through.
- **`ambient_context`'s two-part rule** — canonicalize to enforce containment,
  but pin the caller's spelling so macOS's `/var` → `/private/var` symlink never
  reaches an assertion — with the reasoning written at the surprising line.
- **The corpus-fidelity test** (`copied_prompt_corpus_matches_the_shipped_tree`)
  and its recorded neuter→red→restore, which distinguishes "the copy is complete"
  from "the E2E happened to pass".
- **Honest reporting.** `inventory.md`'s post-change table is explicitly empty
  with the reason named, and `plan.md` marks AC 4 "unstarted (blocked)" rather
  than "partial". The `junit-metrics.ts` script reproducing the baseline table to
  the decimal before being trusted is the correct order of operations.
- **The `wrap_watchdog_timeout.rs` `## Budget sizing` module doc**, which states
  the shared derivation once and leaves each site to name only its own
  fixture-specific margin — and the coordination note flagging the one budget
  whose margin shrinks under the 2026-08-31 startup-stall spec.

## Recommended order of work

1. Implement Required behavior 1's environment scrub in `build()` (finding 1),
   renaming the `CLAUDINE_PROBE_CONTROL` control first, and add the three
   positive tests. Re-run the three leak probes above; all must stay green.
2. Add `bin_exe` to the guard's detector and resolve `wrap_ctrl_c_windows.rs`
   (finding 4).
3. Pre-read the Windows-compiled test subset for bare-name utility use, and
   decide the `inherit_no_env` + Windows console-variable question (finding 3).
4. Drop the manual `TERM_WIDTH` from `wrap_basics.rs`, regenerate its snapshot,
   and record the change against AC 5 (findings 1, 7).
5. Commit, push, and collect the three green runs. Fill the `inventory.md`
   post-change table and close AC 4 and AC 6 (finding 2). Record any miss with
   its cause.
6. Optional but cheap, and worth doing while the context is warm: the roll-up
   artifact (6), the post-`command()` scan or wrapper type (5), the `TMPDIR`
   assertion (9), and deleting `IN_SCOPE` (8).
