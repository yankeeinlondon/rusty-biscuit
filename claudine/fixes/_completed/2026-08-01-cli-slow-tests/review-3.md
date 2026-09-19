---
$schema: feature-review.yaml
ready: false
agent: claude/default
created: 2026-09-06T17:25:46-07:00
spec: 2026-08-01-cli-slow-tests/spec.md
implemented: true
implemented_by: claude/default
log: claudine/fixes/2026-08-01-cli-slow-tests/log.md
description: A **fix** review of `2026-08-01-cli-slow-tests/spec.md`
fix: 2026-08-01-cli-slow-tests/review-3.md
previous: 2026-08-01-cli-slow-tests/review-2.md
---

# Review 3 — `2026-08-01-cli-slow-tests`

## Verdict

**Not production ready**, and — the part that matters most for this iteration —
**nothing in the implementation has changed since review 2.**

This review was commissioned on the understanding that review 2's findings had
been implemented. They have not been. No file under `claudine/cli/` or
`.claude/skills/` has been modified since review 2 was drafted:

```bash
find claudine/cli .claude/skills -newermt "2026-09-06 16:30" -type f
#   (no output)
```

The only two files touched after review 2 landed are `spec.md` (its
`review_iterations` bumped to `2`) and `plan.md`, which gained an honest
**"Open spec gap"** section restating review 2's finding 1 and explicitly
declining to fix it:

> It is deliberately **not** implemented here: it is Phase 1 scope, and
> reopening the builder inside the closing verification phase would silently
> widen this phase.

That is a defensible process argument about *which phase* owns the work. It is
not a reason the fix is shippable. Review 2's two blockers therefore both stand
verbatim:

1. **Required behavior 1's environment-inheritance contract is unimplemented**,
   and I independently reproduced **four** distinct leaks on this branch — one
   more family than review 2 demonstrated (finding 1).
2. **Acceptance criterion 4 has zero evidence.** Nothing is committed, so no
   post-change JUnit artifact exists and AC 6's CI half rides on it (finding 2).

The work that *is* done remains high quality, and this review does not
re-litigate it. `just test-cli` is green (2397 passed / 10 skipped, 13.9 s) and
`just lint` is clean across all four claudine crates.

One correction to review 2 is recorded below (finding 7): its finding 9
overstated the `TMPDIR` hazard. The suite does not go green-and-slow — one test
catches it. I measured the cost anyway, and it is large.

## What was verified for this review

Every row below was executed on this branch during this review, not carried
forward from review 2.

| Check | Result |
|---|---|
| Code changed since review 2 | **none** — `find … -newermt` returns nothing under `claudine/cli` or `.claude/skills` |
| `just test-cli` (claudine area) | green — 2397 passed, 10 skipped, 13.9 s |
| `just lint` (claudine area) | green — claudine, claudine-contract, claudine-cli, claudine-gen |
| Leak probe: `CLAUDINE_TIMEOUT=0.3s` | **3 migrated timeout tests fail** |
| Leak probe: `GIT_DIR` / `GIT_WORK_TREE` | **the hermeticity proof itself fails** |
| Leak probe: `FORCE_COLOR=1` | **1 migrated test fails**, leaking the frontmatter it must withhold |
| Leak probe: `COLUMNS=44` | **5 migrated tests fail** — a render input review 2 did not demonstrate |
| Windows cross-check (`cargo check --target x86_64-pc-windows-msvc -p claudine-cli --tests`) | **blocked** — `aws-lc-sys` build script needs `windows.h` |
| AC 1 — no migrated file in `SPAWN_ALLOWLIST` | pass — 35 live entries, none of the 29 nor `ctx_launch_anchor.rs` / `propagated_context_fixtures.rs` |
| AC 7 — no `slow-timeout` overrides added | pass — `.config/nextest.toml` unmodified |
| Post-`command()` isolation overrides in the 29 + 2 | none live (`.current_dir(`, `.env("PATH"`, `env_clear()`, `augmented_path`) |
| Guard roll-up visibility | invisible by default; needs `--success-output final` |
| `rust-testing` skill hash | `md hash` matches the stamped `1acc7c1c76b11142-e852f9f6596146b8` |
| `TMPDIR` inside the checkout | 0.096 s → **2.065 s** on one migrated test, suite green except one |

`review-1.md` is still not on disk and not in git history, so its frontmatter
could not be updated. `review-2.md`'s `next`/`implemented` fields and the spec's
`review_iterations: 3` have been set as instructed — see the note at the end of
this document about what `implemented: true` should *not* be read to mean.

## Test rigor by requirement

The relevant levels for this fix are **L1** (in-process or child-process probe),
**CI-evidence** (the spec's own reference environment: four environments × three
green runs, JUnit artifacts), and **cross-platform execution** (the Windows and
WSL2 legs actually compiling and running the code).

| Requirement | Level demanded | Level present | Verdict |
|---|---|---|---|
| RB 1 — hermetic builder: CWD, HOME family, `PATH` | L1 | L1 — `cli_process_fixture.rs` asserts on what a recording provider stub received | **met** |
| RB 1 — environment inheritance (`CLAUDINE_*`, `GIT_*`, render inputs) | L1 | **none** — unimplemented, untested, four families reproduced leaking | **gap (critical)** — finding 1 |
| RB 2 — all 29 binaries migrated | L1 structural | L1 — `spawn_site_guard.rs`, two forms | **met** for those forms; a third live form is invisible — finding 4 |
| RB 3 — guard non-vacuity | L1 unit + manual demonstration | both, three arms, reverted | **met** |
| RB 3 — roll-up observable from a CI log | CI log output | `eprintln!` from a passing test; nextest's `success-output` default is `never` | **gap (medium-low)** — finding 6 |
| RB 4 — timeout budgets | **CI**, explicitly | 10 consecutive local `just test-cli` runs, one macOS host | **level mismatch (high)** — finding 2 |
| RB 5 — shipped feature-review contract | L1 + local `--perf` | both, plus a corpus-fidelity test with a recorded neuter→red→restore | **met** |
| RB 6 — measurement of record | CI JUnit artifacts, three green runs | tooling written and baseline-validated; post-change rows empty | **unstarted (high)** — finding 2 |
| AC 8 — cross-platform | execution on all four CI environments | macOS only; the `#[cfg(windows)]` arms **cannot** be compiled on this host | **level mismatch (high)** — finding 3 |
| RB 7 — docs and skills | n/a | both skills updated in-change, `rust-testing` re-hashed and verified | **met** |

## Findings

### 1. (critical) The environment-inheritance contract is still unimplemented — four families leak

`ClaudineCommandBuilder::build` (`claudine/cli/tests/common/mod.rs:338`) sets
`HOME`, `USERPROFILE`, `APPDATA`, `LOCALAPPDATA`, `PATH`,
`CLAUDINE_RENDEZVOUS_REPORT`, `NO_COLOR`, and removes `HOMEDRIVE`, `HOMEPATH`,
`XDG_CONFIG_HOME`. Required behavior 1's fourth bullet asks for more:

> the builder removes the inherited `CLAUDINE_*` namespace and the
> `GIT_DIR`/`GIT_WORK_TREE`/`GIT_INDEX_FILE`/`GIT_COMMON_DIR`/
> `GIT_OBJECT_DIRECTORY` plumbing family, and pins the rendering inputs
> `TERM_WIDTH`, `COLUMNS`, and `FORCE_COLOR`

A grep for `GIT_DIR`, `TERM_WIDTH`, `COLUMNS`, `FORCE_COLOR` across
`common/mod.rs` returns nothing but a prose mention. Reproduced this review,
each from the `claudine` package area:

```bash
CLAUDINE_TIMEOUT=0.3s just test-cli watchdog
#   3 failed: watchdog_subagent_hang_terminates_and_names_stuck_ids,
#             watchdog_opencode_post_fanout_silence_does_not_kill_prematurely,
#             watchdog_stream_idle_timeout_after_tool_call_hang

GIT_DIR=/tmp/gitprobe/.git GIT_WORK_TREE=/tmp/gitprobe just test-cli ambient
#   1 failed: ambient_context_escape_pins_the_cwd_to_a_test_built_repository
#   left:  .../repo/claudine/cli      right: .../repo

FORCE_COLOR=1 just test-cli non_tty_withholds_yaml
#   1 failed: inline_compose_sequence_mismatch::non_tty_withholds_yaml_but_keeps_guidance

COLUMNS=44 just test-cli
#   5 failed, all in characterization_error_routes
```

Three things sharpen this beyond review 2's account:

- **`COLUMNS` is a fourth leaking family, and it is the widest.** Review 2
  demonstrated `FORCE_COLOR`; `COLUMNS=44` turns *five* migrated
  `characterization_error_routes` tests red. `cli/src/log.rs:34-38` reads
  `TERM_WIDTH` then `COLUMNS` before falling back to 80, so any parent shell
  that exports `COLUMNS` — which interactive shells routinely do — reshapes
  every width-sensitive assertion in the suite. Four migrated files
  (`wrap_basics.rs:266`, `argv_normalization.rs:217,268`,
  `command_routing.rs:60`) already hand-pin `TERM_WIDTH` at the call site;
  that is the requirement asserting itself through a workaround, once per file
  that noticed.
- **The `GIT_*` arm defeats the hermeticity proof itself.** The failing test is
  the one whose whole job is to show that `ambient_context` can only ever anchor
  on a repository the test built. An inherited `GIT_DIR` walks claudine's
  repository discovery somewhere else, and the child never reaches the built
  repo root. The spec's Problem §5 records that this exact family already caused
  a real incident on 2026-08-31.
- **The `CLAUDINE_*` arm has the most expensive failure mode.** It
  re-parameterizes precisely the timeout tests this fix just tightened, so the
  symptom a developer sees is "your machine says the new budgets are wrong".

**Two practical notes for whoever implements it.**

- Scrub `CLAUDINE_*` **by prefix**, not by enumeration. There are 48 distinct
  `CLAUDINE_*` names across `lib/src` and `cli/src` today (the spec says 49 and
  names `CLAUDINE_OPTIONS`, which is a usage-string placeholder in
  `cli/src/argv/partition.rs`, not an env var — do not use the spec's list as
  the implementation's list). Beyond the timeout family the dangerous inherited
  names include `CLAUDINE_BIN`, `CLAUDINE_SYSTEM_PROMPT`, `CLAUDINE_YOLO`,
  `CLAUDINE_INTERACTIVE`, `CLAUDINE_HARVEST`, `CLAUDINE_MAX_ITERATIONS`,
  `CLAUDINE_FAIL_FAST`, `CLAUDINE_SNAPSHOT_ROOT`, `CLAUDINE_SEQUENCE_ROOT`, and
  `CLAUDINE_RAW_STREAM_DIR`.
- Review 2's landmine still applies: `cli_process_fixture.rs:199` uses
  `CLAUDINE_PROBE_CONTROL` as its "the parent's environment really does reach
  the child" control and asserts at line 206 that the child received it. That
  assertion currently *encodes* the absence of a `CLAUDINE_*` scrub. Rename it
  to a non-`CLAUDINE_` name first. (`CLAUDINE_PROBE_CAPTURE` at line 81 is set
  *after* `build()`, so the spec's "removal is per key at build time" rule
  leaves it alone — but that rule needs its own positive test.)

Three tests this deserves, none of which exist: an inherited
`CLAUDINE_STEP_TIMEOUT` does not reach the child; an inherited `GIT_DIR` does
not; a call site that sets `CLAUDINE_STEP_TIMEOUT` after taking the builder
still wins.

### 2. (high) Acceptance criterion 4 is still unstarted; the Outcome table has no evidence

Nothing on this branch is committed (`git status` shows 36 modified and 4
untracked source files), so no post-change JUnit artifact exists. Against the
spec's requirement of 4 environments × 3 consecutive green runs, the fix has:

- **zero of the required 12 data points**;
- one local `NEXTEST_PROFILE=ci` macOS run standing in for all of them
  (0 tests ≥ 5 s, 0 non-timeout ≥ 2 s, 28.5 s / 7.0 s serial sums), which the
  spec explicitly forbids as proof: *"Local runs are for attributing cost, never
  for proving a target."*
- a validated measurement script (`junit-metrics.ts`) that reproduces the
  baseline table to the decimal — genuinely good preparation, and it means this
  item closes by pushing rather than by more engineering.

AC 6 inherits the same gap. The WSL2 leg is the one that matters: the baseline
shows these test shapes running 20–50× slower there, and
`sequence_per_step_step_timeout_override`'s `0.5s` step budget is the value the
plan itself flags as having the thinnest margin under contention.

`inventory.md`'s post-change table remains explicitly empty with the reason
named. That honesty is the right posture and is not a criticism — but "ready for
production" cannot be asserted over it.

### 3. (high) The Windows arms cannot be compiled on this host — CI is the only compiler

Review 2 flagged that the `#[cfg(windows)]` code had never been compiled. I
tried to close that gap locally and **it is not closeable**:

```bash
cargo check --target x86_64-pc-windows-msvc -p claudine-cli --tests
#   error occurred in cc-rs: aws-lc-sys … jitterentropy-base-windows.h:49:
#   fatal error: 'windows.h' file not found
```

The `x86_64-pc-windows-msvc` target is installed, but a transitive native
dependency needs the Windows SDK headers, so `cargo check` cannot even reach the
test crate. The `windows-latest` CI leg is therefore the *first* compiler these
arms will ever see, which raises the cost of the first push from "read a table"
to "iterate on a red leg".

Concretely, 13 of the 29 migrated binaries have no `#![cfg(unix)]` file gate and
so compile on Windows — `inline_compose_hash`, `mcp_cli`,
`shipped_prompt_contract`, `wrap_compose_validation`, `wrap_inline_compose`,
`wrap_basics`, `command_routing`, `argv_normalization`, `hooks_cli`,
`contextual_errors`, `inline_compose_sequence_mismatch`, `handle_repo_config`,
`characterization_error_routes` — plus the new `cli_process_fixture.rs`, which
is where all the new Windows-only code lives (`minimal_system_path()`'s
`SystemRoot` arm, the `.cmd` recording stub, and the `%SystemRoot%\System32`
assumptions). AC 8's entire cross-platform guarantee rests on a file that has
never been type-checked for its target.

Two things worth pre-empting with a static read before the push, both cheap:

- The spec is explicit that `System32` resolves **none** of `sh`, `cat`,
  `sleep`, or `git`. Any Windows-compiled test whose fixture shells out by bare
  name now fails where it previously inherited the runner's full `PATH`. (The
  Unix-side audit is clean: the bare `sleep 30` in `sequence_schema.rs:298` and
  the `cat`/`printf` uses in `wrap_structured_stream.rs` all sit in
  `#![cfg(unix)]` files and resolve under `/usr/bin:/bin`.)
- `inherit_no_env()` calls `env_clear()`, which on Windows takes `PATHEXT`,
  `COMSPEC`, and `SystemRoot` with it — exactly what `.cmd` stub resolution and
  the `SystemRoot` fallback need. The module docs warn about it and the only
  current caller is `#[cfg(unix)]`, so it is latent rather than broken. Re-adding
  those three inside `build()` when `inherit_env == false` on Windows would keep
  the tightening knob usable instead of a trap.

### 4. (medium) A third live spawn form is still invisible to the guard

Unchanged from review 2 and re-verified: `wrap_ctrl_c_windows.rs:130` obtains
the binary with `biscuit_test_harness::bin_exe!("claudine")` and spawns it with
`std::process::Command::new`, handing the child a **full host `PATH`** (built
inline at lines 125–128) and an untouched `CLAUDINE_*` / `GIT_*` environment.

The file's own module docs argue at length that it is an ordinary **Level 1**
test. It carries no `level2_`/`level3_`/`real_` prefix, so `excluded()` does not
skip it, and it has no `SPAWN_ALLOWLIST` entry — yet the guard reports zero
sites for it, because `spawn_sites()` (`spawn_site_guard.rs:256`) only knows
`cargo_bin` and `claudine_bin`. Because the file is `#[cfg(windows)]`, no macOS
or Linux run will ever surface it.

Consequences: the burn-down census under-reports, and `bin_exe!` is now the
frictionless way to add a non-hermetic L1 spawn — the exact regrowth the guard
exists to prevent. (`sequence_ctrl_c_windows.rs:181` uses `claudine_bin()` and
*is* correctly allow-listed, which shows the mechanism works when the form is
known.)

Fix: add `FORM_BIN_EXE` keyed on the `bin_exe` identifier with the same
`names_claudine` literal check — the macro's argument is a plain `"claudine"`
literal, so the existing helper works unchanged — add a detector unit test
alongside the existing two, and then either migrate the file or give it an
allow-list entry naming its reason (it needs `CREATE_NEW_PROCESS_GROUP`, which
`assert_cmd` does not expose).

### 5. (medium) Isolation can still be defeated after `command()`, and the guard is blind to it

`build()` hands back a bare `assert_cmd::Command`. Nothing stops a future call
site from writing `.current_dir(repository_root())` or
`.env("PATH", augmented_path(&dir))` on it, reinstating both leaks the fix
removed — and the spawn form stays perfectly legitimate, so the guard says
nothing. `augmented_path` is still `pub` in `common`, reachable without going
through `host_path()`.

I re-checked all 29 migrated binaries plus `ctx_launch_anchor.rs` and
`propagated_context_fixtures.rs` for `.current_dir(`, `.env("PATH"`,
`env_clear()`, and `augmented_path`: **no live violation**. This is about
whether "hermetic by construction" survives the next six months, not about a
bug today.

Two options, in increasing strength:

- Cheap: extend the source scan to flag those three forms in governed files, on
  the same allow-list mechanics. The escapes already cover every legitimate
  need, so false positives should be near zero.
- Stronger: return a small wrapper type exposing `arg`/`args`/`env`/`assert`/
  `output`/`timeout` and nothing else, funnelling CWD and `PATH` through builder
  methods. A mechanical pass over 29 files buys a type instead of a convention.

### 6. (medium-low) The burn-down roll-up still never reaches a CI log

Required behavior 3 asks that the roll-up be "observable from a CI log without
re-running a census". Demonstrated this review:

```bash
just test-cli l1_tests_spawn
#   PASS … l1_tests_spawn_claudine_through_the_fixture_builder   (no roll-up)

just test-cli "l1_tests_spawn --success-output final"
#   spawn-site burn-down: 169 raw sites in 35 allow-listed files (of 169 scanned)
#     == outside this fix's scope: 35 file(s), 169 site(s)
```

`.config/nextest.toml` sets no `success-output` in either profile, and nextest's
default is `never`, so the roll-up is visible only on runs where the guard
*fails* — precisely when nobody needs the census.

Write it to `$BISCUIT_JUNIT_STAGE_DIR` (or `target/nextest/ci-reports`) as a
small artifact, the way `backend-executions.jsonl` is written; that fits the
repo's existing evidence conventions better than a `success-output` override.

### 7. (low — **correcting review 2**) The `TMPDIR` hazard is real and costly, but it is not silent

Review 2's finding 9 predicted that a `TMPDIR` pointing inside the checkout
would make "the entire fix silently revert with a green suite". Measured this
review, that is **not** what happens — but the cost half is worse than stated.

```bash
just test-cli agents_and_commands_route                       # 0.096 s  PASS
TMPDIR=<checkout>/target/tmpdir-probe just test-cli …         # 2.065 s  PASS
```

A 21× regression on one migrated test, on an idle 16-core Mac — this is the
14.3 s Ubuntu baseline coming straight back under CI contention. The suite is
*not* fully green, though: `default_command_pins_cwd_home_and_the_minimal_system_path`
fails with a clear message, because claudine discovers the checkout repository
and starts the agent at its root instead of the fixture `cwd`:

```
the default command must launch from the fixture cwd, not the checkout
  left:  /Users/…/fix-cli-slow-tests
  right: /Users/…/target/tmpdir-probe/fixture-default-shape-…/cwd
```

So the fixture self-test catches it — incidentally, as a side effect of asserting
on CWD, not by design, and with no comment saying so. The failure mode a
developer actually meets is "one unrelated-looking fixture test is red and
everything else got slower", which is a poor diagnostic for the cause.

Recommendation stands but is cheaper than review 2 implied: one assertion in
`CliProcessFixture::named` that the workspace root is not under the repository
root, naming `TMPDIR` in its message. It composes naturally with the containment
check `ambient_context` already performs, and it turns an accidental guard into
an intentional one.

### 8. (low) `IN_SCOPE` is still dead vocabulary kept alive by its own unit test

`spawn_site_guard.rs:55–62` keeps a constant no production path uses, with a
six-line doc comment explaining that its only remaining consumer is
`reconciliation_reports_unlisted_sites_stale_and_unexplained_entries`, which
"needs *some* reason string". That test can use a string literal. The constant
plus its justification is what the monorepo's comment rules ask to delete: a
future burn-down will invent its own vocabulary anyway.

### 9. (low, new) The structural gates now carry the fixture's whole dependency surface

`test_placement.rs` was refactored to share the sanitizer, which is the right
call — but it reaches it with `mod common;`, which compiles `common::wrap` (the
`claudine::mcp::types` / `chrono` / `serde_json` fixture surface) and, on Unix,
`common::pty` (expectrl) into a binary whose entire job is scanning text for
substrings. Same for `spawn_site_guard.rs`.

`source_scan.rs` has no dependencies beyond `std`. Both gates could take it
directly:

```rust
#[path = "common/source_scan.rs"]
mod source_scan;
```

That keeps the two cheapest binaries in the suite cheap, and keeps a structural
gate from failing to build because a fixture helper's dependency broke.

## What is done well

Recorded again so it is not re-litigated, and because none of it regressed:

- **The probe design in `cli_process_fixture.rs`** — asserting on what a
  recording provider stub actually received rather than on the builder's fields.
  Naming the stub `claude`, a provider a developer machine plausibly has
  installed, makes "the stub ran" itself the proof that no host install won
  selection.
- **The three named escapes and their discipline.** Every escape call site
  across the migrated files carries a comment naming the tool or the proof it
  depends on. That follow-through is unusual.
- **`ambient_context`'s two-part rule** — canonicalize to enforce containment,
  pin the caller's spelling so macOS's `/var` → `/private/var` symlink never
  reaches an assertion — with the reasoning written at the surprising line.
- **The corpus-fidelity test** (`copied_prompt_corpus_matches_the_shipped_tree`)
  and its recorded neuter→red→restore.
- **`wrap_watchdog_timeout.rs`'s `## Budget sizing` module doc**, which states
  the shared derivation once and leaves each site to name only its own
  fixture-specific margin.
- **Honest reporting throughout.** `inventory.md`'s post-change table is
  explicitly empty with the reason named; `plan.md` marks AC 4 "unstarted
  (blocked)" rather than "partial", and now carries the unimplemented RB 1
  bullet as a named open gap rather than quietly omitting it.

## Recommended order of work

Unchanged from review 2 in substance; step 1 has grown one family.

1. Implement RB 1's environment scrub in `build()` — `CLAUDINE_*` by prefix, the
   five `GIT_*` plumbing keys, and `TERM_WIDTH`/`COLUMNS`/`FORCE_COLOR` pinned
   or removed. Rename `CLAUDINE_PROBE_CONTROL` first. Add the three positive
   tests. Re-run all four leak probes from finding 1; all must stay green.
2. Add `bin_exe` to the guard's detector and resolve `wrap_ctrl_c_windows.rs`
   (finding 4).
3. Statically read the 13 Windows-compiled migrated binaries plus
   `cli_process_fixture.rs` for bare-name utility use, and decide the
   `inherit_no_env` + Windows console-variable question (finding 3). Local
   compilation is not available.
4. Drop the manual `TERM_WIDTH` from `wrap_basics.rs` and the two other files
   once the builder owns the width, regenerate the snapshot, and update the AC 5
   deviation note.
5. Commit, push, collect the three green runs, fill `inventory.md`'s post-change
   table, and close AC 4 and AC 6 (finding 2). Record any miss with its cause.
6. Cheap while the context is warm: the roll-up artifact (6), the
   post-`command()` scan or wrapper type (5), the `TMPDIR` assertion (7),
   deleting `IN_SCOPE` (8), and the `#[path]` include (9).

## Note on this document's frontmatter

`review-2.md`'s `implemented` field has been set to `true` as instructed by the
review workflow. It should be read as "this review has been processed into a
successor", **not** as "review 2's findings were implemented" — they were not,
as the first section of this document establishes. If that field is load-bearing
anywhere, it needs a different value.
