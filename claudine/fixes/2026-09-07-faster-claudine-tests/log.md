---
fix: 2026-09-07-faster-claudine-tests
phase: 10
created: 2026-09-07
---

# Implementation log

## Phase 1 — Land the predecessor and open the attribution window

**Outcome: partially complete. The local half is done and green; the CI half is
blocked on operator action and is recorded as pending, not as passing.**

Phase 1 is a gate, and it has not fully closed. Phases 4–10 remain blocked per
the plan's assumption 1. Phases 2 and 3 are document-only and may proceed.

### Grounding facts re-checked

The plan requires a phase to stop and re-derive rather than proceed when a
grounding fact no longer holds. Two moved:

- **The predecessor's fix directory is now `claudine/fixes/_completed/2026-08-01-cli-slow-tests/`.**
  The plan describes it at `claudine/fixes/2026-08-01-cli-slow-tests/`. It was
  archived to `_completed` after the plan was written. Nothing else about the
  fact changed: the directory is tracked, and all of `spec.md`, `plan.md`,
  `log.md`, `inventory.md`, `deferred-performance.md` and `junit-metrics.ts` are
  present. Paths in this fix's documents point at the `_completed` location.
- **The branch is 214 commits ahead of `main`, not 191, and `main` has diverged
  by one commit.** Commit count is not load-bearing for any Phase 1 decision;
  the sequencing claim it supports — that the predecessor has not landed — still
  holds.

Confirmed unchanged:

- `fix/cli-slow-tests` @ `9fc5151a0` carries the predecessor's three
  implementation commits: `cd3a28115` (hermetic spawn fixture and structural
  guards), `3e318802d` (L1 migration to `CliProcessFixture`), `5b5b92bfb`
  (planning set).
- `git diff main -- .config/nextest.toml` is **empty**. No override, retry or
  tier change has been introduced (AC5, AC7).

### Local gates

Run from the `claudine` package area at `9fc5151a0`, working tree carrying only
this fix's documents plus untracked planning directories. Verbatim output is in
[`baseline/local-gates/`](baseline/local-gates/).

| Gate | Result | Wall | Log |
|---|---|---:|---|
| `just test` | 6861 passed, 11 skipped, **exit 0** | 36.0 s | `just-test.log` |
| `just test-cli` | 2489 passed, 10 skipped, **exit 0** | 18.1 s | `just-test-cli.log` |
| `just test-l2` | `claudine-cli` 237 passed; `claudine-gen` 3 passed, **exit 0** | 66.8 s + 2.7 s | `just-test-l2.log` |
| `just lint` | **exit 0** | 48.2 s | `just-lint.log` |
| `just check-windows` | **exit 0**, 5 warnings | 1 m 37 s | `just-check-windows.log` |
| `just ci-local --lint-only` | 28/28 gates passed, **exit 0** | ~28 min | `ci-local-lint-only.log` |
| `just ci-local` | 55/55 gates passed, **exit 0** | ~42 min | `ci-local.log` |

Both `ci-local` runs selected **27 packages** (class=package, full_scope=false)
from 79 files changed against `origin/main` (`a9e88c069`) — the branch's whole
affected scope, not just `claudine`. Zero `FAIL`, `TIMEOUT`, `SIGSEGV` or
`LEAK-FAIL` lines across the run.

`git diff main -- .config/nextest.toml` and
`git diff origin/main -- .config/nextest.toml` are both empty at the end of the
phase as well as the start: no override, retry, tier change or disabled
assertion was introduced (AC5, AC7).

**Deliberate deviation from the plan's ordering.** The plan lists `just lint`
first. Tests were run first instead: `just lint` is
`cargo clippy --all-targets`, which shares the target directory with nextest and
leaves metadata-only fingerprints that make cargo consider the test targets
fresh. The first test run after a lint therefore re-executes the previously
built binaries and reports green while silently omitting any test added since.
Running tests first removes the hazard rather than working around it. The
observed counts corroborate that nothing was stale: `just test-cli` reports
2489, above the 2397 in the staged report from 2026-09-06, so the newly added
binaries were built and run.

Local numbers are **attribution only** and establish no target (spec RB5).

### `just check-windows` — the predecessor's finding-3 compile half

**This closes, on this host, for the `x86_64-pc-windows-gnu` target.**

`deferred-performance.md` item 2 recorded that the `#[cfg(windows)]` arms had
never been compiled anywhere, because
`cargo check --target x86_64-pc-windows-msvc` fails inside `aws-lc-sys`, whose
`jitterentropy-base-windows.h` needs Windows SDK headers this macOS host lacks.
The area's `just check-windows` recipe uses the **mingw** target instead, with
`-Wa,-mbig-obj`; that path has no such dependency problem. Its Rust standard
library was not installed on this host, so `rustup target add
x86_64-pc-windows-gnu` was run before the check. The recipe then completed
`cargo check -p claudine -p claudine-cli --tests --target x86_64-pc-windows-gnu`
in 1 m 37 s with exit 0.

So the arms named in the predecessor's "what is unverified anywhere" list —
`restore_windows_console_variables`' three `.env()` calls, the Windows arm of
`inherit_no_env_keeps_the_defaults_and_drops_everything_else`,
`minimal_system_path()`'s `%SystemRoot%\System32` arm and the `.cmd` recording
stub — now compile.

**What this does not close.** Compiling is not running. Whether a cleared
Windows environment is missing a fourth variable claudine needs is still a
runtime question that only the `windows-latest` leg answers. mingw is also not
MSVC: anything MSVC-specific in the native dependency graph remains unproven.
The predecessor's honest risk statement stands unchanged.

**New finding, not fixed here.** The Windows check emits three unused-import
warnings that the host build does not:

- `claudine/cli/tests/wrap_basics.rs:7` — `use std::fs;`
- `claudine/cli/tests/wrap_basics.rs:9` — `use common::wrap::*;`
- `claudine/cli/tests/compose_caller_file_provenance.rs:5` — `write_executable`

These are the residue of `#[cfg(unix)]`-gated cases. They are warnings, not
errors, so the gate is green. They are recorded rather than fixed: Phase 1 is a
gate phase, and both files are in scope for Phase 5, which is where the
correction belongs alongside the change that motivates it.

### `junit-metrics.ts` — forked and hardened

[`junit-metrics.ts`](junit-metrics.ts) forks the predecessor's script.
[`baseline/expectations.json`](baseline/expectations.json) declares the four-leg
expectation and [`baseline/README.md`](baseline/README.md) documents the layout
and the collection recipe.

**Why a fork and not an extension.** The predecessor's script could not be a
gate. It matched `<testcase>` with a single regex requiring
`name=… classname=… time=…` in exactly that order, so any other attribute
ordering or a self-closing element was silently dropped from every total; it
read one hard-coded path per environment rather than the staging tree that CI
actually uploads; it printed `warning: no artifact for <env>` and carried on;
and it exited 0 unconditionally.

**What the fork does differently:**

- Reads the `_stage_junit` staging tree — `<tier>/<package>.xml` plus
  `manifest.jsonl` — so `exit_code`, `duration_s` and `report_present` are
  available. A tree without a manifest is rejected as not-a-staging-directory.
- Replaces the regex with an attribute-order-independent tag scan that tracks
  element depth, so a `<failure>` quoted inside `<system-out>` is prose rather
  than an outcome.
- Emits the plan's **three separate cost columns**: build/setup
  (`duration_s` − `<testsuites time>`), runner elapsed (`<testsuites time>`),
  and summed duration (Σ `<testcase time>`).
- Exits **1** on any violation and **2** on a usage error.

The six rejection classes the plan names, plus one:

| Class | Detected as |
|---|---|
| malformed reports | unterminated tag/comment/attribute, mismatched or unclosed elements, no `<testsuites>` root, `<testcase>` outside the root, declared-vs-enumerated test-count mismatch |
| missing expected artifacts | absent leg directory, absent `manifest.jsonl`, no manifest record for an expected cell, `report_present: false`, manifest naming an XML absent from the artifact |
| missing expected tests | `requiredTests` identity or bare name not executed on a leg |
| duplicate identities | one `<suite>::<case>` listed twice in a report |
| invalid durations | missing, non-numeric or negative `time`; manifest `duration_s` below the run's own elapsed beyond whole-second rounding |
| failed runs | non-zero `exit_code`, an enumerated `<failure>`/`<error>`, or a report declaring more failures than it enumerates |
| *(added)* timeout-floor misses | opt-in via `enforceTimeoutFloors`, so a `TIMEOUT_TESTS` miss cannot pass as a printed remark |

A declared **pending** leg is reported as pending; an undeclared missing leg is
a violation. That distinction is what keeps "blocked legs are named as pending
rather than assumed" mechanical instead of editorial.

#### Verification

46 tests in [`junit-metrics.test.ts`](junit-metrics.test.ts), run with
`npx tsx --test junit-metrics.test.ts` — 46 passed, 0 failed. `node:test`
rather than Vitest because the monorepo ships no JavaScript test runner and
this fix is not the place to introduce one; `tsx` is already how the
predecessor's script was invoked.

**Real-artifact coverage.** `fixtures/nextest-l1-excerpt.xml` is a genuine
31-case excerpt cut from a real staged `L1/claudine-cli.xml`, not a
hand-written imitation. A passive corpus test parses every `.xml` under
`fixtures/` and reconciles each one's declared count against its body. A
further test reads the host's own full staged report when one is present.

**End-to-end through the normal invocation path**, against the real 2397-case
report from the 2026-09-06 `NEXTEST_PROFILE=ci` run:

```text
| Environment      | Tier | Package       | Build/setup | Runner elapsed | Summed duration | Tests | Failures | Skips |
| `ubuntu-latest`  | L1   | `claudine-cli`|     75.5 s  |       170.5 s  |        170.4 s  |  2397 |        0 |     0 |
GATE EXIT=0
```

and the same artifact with a stricter expectation:

```text
2 violation(s):
  [missing-artifact] windows-latest: no artifact directory; a leg is pending only when declared pending
  [missing-test] ubuntu-latest: required test a_test_that_stopped_running was not executed
GATE EXIT=1
```

Summed duration tracking elapsed almost exactly is expected here: the CI
profile pins `claudine-cli`'s L1 group to one thread.

**Non-vacuity, by neuter.** Three detectors were disabled one at a time, the
suite re-run, and the file restored between each:

| Neuter | Named failure | Result |
|---|---|---|
| `if (false && seen.has(…))` — duplicate detection | `rejects_a_report_listing_one_identity_twice` | 45 pass / 1 fail |
| `requiredNumber` returns 0 instead of throwing | `rejects_a_testcase_with_no_duration_rather_than_scoring_it_zero` | 45 pass / 1 fail |
| `const insideText = false` — `<system-out>` guard | `a_failure_element_quoted_inside_captured_output_is_not_a_failure` | 45 pass / 1 fail |

After restoring, `sha256(junit-metrics.ts)` is
`956b849d7fce55b7cef05e7956a1bc382ee5f970ee384e482e43d6cdc40057d2`, identical
to the pre-neuter hash, and the suite is 46/46 again.

A `time` violation is classified by exception type (`InvalidDuration`), not by
matching the word "time" in a message, so a malformed document that happens to
say "timed out" is still reported as malformed. That distinction has its own
test.

#### Requirement-to-test mapping

Phase 1 changed one behavior: the gate script. Every clause of its requirement
maps to named tests.

| Requirement clause | Tests |
|---|---|
| reads the four-leg baseline | `rejects_a_missing_environment_leg`, `reports_a_declared_pending_leg_as_pending_instead_of_missing`, `wsl2_gets_one_extra_second_on_every_timeout_floor` |
| emits the table `inventory.md` will hold | `passes_the_real_shipped_report`, `repeated_reads_of_one_tree_produce_an_identical_table` |
| three separate cost columns | `passes_the_real_shipped_report` (asserts build/elapsed/summed are all distinct and independently derived) |
| rejects malformed reports | `rejects_a_document_that_is_not_a_junit_report`, `rejects_an_unterminated_tag`, `rejects_a_truncated_download_with_an_unclosed_element`, `rejects_mismatched_close_tags`, `rejects_a_testcase_outside_any_testsuites_root`, `rejects_a_report_whose_declared_test_count_disagrees_with_its_body`, `a_malformed_manifest_is_a_violation_not_a_crash`, `rejects_manifest_lines_that_are_not_json`, `rejects_manifest_records_with_the_wrong_field_types` |
| rejects missing expected artifacts | `rejects_a_missing_environment_leg`, `rejects_a_tree_with_no_staging_manifest`, `rejects_a_manifest_record_whose_report_was_never_emitted`, `rejects_a_manifest_naming_an_xml_that_is_absent_from_the_artifact`, `rejects_an_expected_cell_with_no_manifest_record`, `rejects_a_missing_artifact_directory_outright` |
| rejects missing expected tests | `rejects_a_run_that_silently_stopped_executing_a_required_test`, `matches_a_required_test_given_as_a_full_identity` |
| rejects duplicate identities | `rejects_a_report_listing_one_identity_twice` |
| rejects invalid durations | `rejects_a_testcase_with_no_duration_rather_than_scoring_it_zero`, `rejects_a_non_numeric_duration`, `rejects_a_negative_duration`, `accepts_a_zero_duration_boundary`, `rejects_a_manifest_duration_below_the_runs_own_elapsed`, `tolerates_whole_second_manifest_rounding_below_elapsed`, `classifies_a_bad_duration_apart_from_other_malformation` |
| rejects failed runs | `rejects_a_nonzero_exit_code_even_when_every_case_passed`, `rejects_an_enumerated_failure`, `rejects_a_report_declaring_more_failures_than_it_enumerates`, `counts_a_skipped_case_as_a_skip_and_not_a_failure`, `a_real_failure_element_marks_its_case` |
| "a script that prints a miss and exits 0 is not a gate" | `main_exits_zero_on_a_clean_tree_and_one_on_a_violation`, `main_exits_two_on_a_usage_error`, `a_timeout_floor_miss_is_only_printed_until_enforcement_is_asked_for` |
| representation variants | `counts_testcases_regardless_of_attribute_order_or_quoting` (attribute order, single quotes, self-closing), `decodes_xml_entities_in_test_identities`, `a_failure_element_quoted_inside_captured_output_is_not_a_failure`, `ignores_blank_manifest_lines`, `a_test_exactly_on_its_bound_is_not_a_miss` |
| passive corpus over shipped artifacts | `every_shipped_fixture_parses_and_reconciles_its_own_declared_count` |
| end-to-end via the real artifact and normal invocation | `passes_the_real_shipped_report`, `reads_the_hosts_own_staged_report_when_one_is_present`, plus the two CLI transcripts above |

Which of these are regressions in the strict sense — failing against the
predecessor's implementation and passing against this one — was checked by
running its regex over each variant rather than assumed:

| Variant | Predecessor's regex |
|---|---|
| nextest's own attribute order | matched |
| `time`-first attribute order | **dropped** |
| single-quoted attributes | **dropped** |
| self-closing, nextest's order | matched |
| self-closing, reordered | **dropped** |

So `counts_testcases_regardless_of_attribute_order_or_quoting` is a genuine
regression test; self-closing elements alone were not the defect, attribute
order was. A dropped `<testcase>` is invisible in the old script — it lowers
every total silently, which reads as a speed-up.

The outcome tests are a different category: the predecessor never parsed
`<failure>`, `<error>` or `<skipped>` at all, so it could not distinguish a
green run from a red one. `a_real_failure_element_marks_its_case` and
`a_failure_element_quoted_inside_captured_output_is_not_a_failure` cover new
capability, not a repaired defect.

### Blocked: the CI half

Three plan tasks cannot be executed in this session and are **left unchecked**:

1. **Hand off to the operator for commit, push and merge; record the merge SHA.**
   `CLAUDE.md` makes committing, pushing and merging operator actions, and all
   commits must be OpenPGP-signed. This session is non-interactive and cannot
   supply a signing passphrase; attempting a signed commit would hang rather
   than fail. Nothing was committed or staged.
2. **Collect three consecutive green CI runs on `main` across `ubuntu-latest`,
   `macos-latest`, `windows-latest` and `wsl2-ubuntu`.** Blocked by 1 — there is
   no post-merge run to read. This is a committed-state problem, exactly as the
   predecessor's `deferred-performance.md` describes; no local run substitutes,
   and none is offered as one.
3. **Store the JUnit artifacts under `baseline/<run-id>/`.** Blocked by 2. The
   directory layout, the collection recipe, and the gate that will read them are
   in place and tested; only the artifacts are missing.

### What the operator needs to do

```bash
# 1. from the worktree, with signing available
git add claudine/fixes/2026-09-07-faster-claudine-tests
git commit          # signed; author Ken Snyder <ken@ken.net>
git verify-commit HEAD
git push

# 2. merge the predecessor to main, then, for each of three consecutive
#    green runs, per leg:
run=<run-id>
for env in ubuntu-latest macos-latest windows-latest wsl2-ubuntu; do
  gh run download "$run" -R yankeeinlondon/rusty-biscuit \
    -n "junit-claudine-cli-L1-$env" \
    -D "claudine/fixes/2026-09-07-faster-claudine-tests/baseline/$run/$env"
done

# 3. the gate
npx tsx claudine/fixes/2026-09-07-faster-claudine-tests/junit-metrics.ts \
  "claudine/fixes/2026-09-07-faster-claudine-tests/baseline/$run" \
  --label "run $run" \
  --expect claudine/fixes/2026-09-07-faster-claudine-tests/baseline/expectations.json
```

Every intervening failed attempt is recorded with its cause. Selecting only the
successful attempts is disallowed.

### Validation checkpoint 1 — not passed

| Requirement | Status |
|---|---|
| Three green runs on four legs exist | **pending** — blocked on merge |
| Their artifacts are stored | **pending** — blocked on the above |
| `junit-metrics.ts` reproduces the baseline table from them | **ready, unexercised on CI data** — proven against a real local report |
| Predecessor's two deferrals resolved in writing | **item 2 open; item 1's compile half closed** — see `deferred-performance.md` |
| Blocked legs named as pending rather than assumed | **done** — all four legs are named pending here and in `baseline/README.md` |

### Findings carried forward

Recorded here rather than acted on, because Phase 1 is a gate phase and the
plan places both fixes in later phases.

1. **Three Windows-only unused-import warnings** in `wrap_basics.rs` and
   `compose_caller_file_provenance.rs` (detailed above). → Phase 5, which edits
   both files.
2. **The `rust-testing` skill documents no cross-compilation route.** Nothing
   under `.claude/skills/rust-testing/` mentions `check-windows`,
   `x86_64-pc-windows-gnu`, or the msvc/mingw distinction — which is why the
   predecessor concluded its Windows arms were uncompilable off CI. → Phase 10,
   which owns skill updates. No skill file was changed in this phase.

Phases 4–10 stay blocked. Phases 2 and 3 are document-only and unblocked.

---

## Phase 2 — Reconciled inventory (RB1, AC1)

**Outcome: complete. Validation checkpoint 2 passed.** Document-only: no Rust
package source was changed. Phases 4–10 remain blocked on Phase 1's CI
checkpoint.

Deliverables: [`inventory.md`](inventory.md), [`families.json`](families.json),
[`inventory-reconciler.ts`](inventory-reconciler.ts) with
[70 tests](inventory-reconciler.test.ts), and the enumeration substrate under
[`enumeration/`](enumeration/).

### Grounding facts re-checked

Four of the plan's grounding facts moved. Each is superseded by a mechanical
measurement, and each is recorded in `inventory.md` next to the number it
replaces.

| Plan said | Measured at `9fc5151a0` |
|---|---|
| ≈7,300 attributes; `lib` 4086 / `cli` 2728 / `rendezvous` 296 / `gen` 159 | 7,400 runner identities and 7,455 source attributes; 4,149 / 2,746 / 274 / 158 |
| spawn burn-down: 170 sites / 168 out-of-scope; 83 governed; isolation governs 32 | **172 / 170; 89 governed; isolation governs 37**, 0 escapes. The guard's own artifact is the authority; the plan's per-file table is right except `level1_structured_error_message.rs` (1 site, not 2), and its own table summed to 173 rather than the 170 it claimed |
| 18 per-test `slow-timeout` overrides (9 default + 9 CI) | **17** (9 + 8), inside 24 override blocks |
| `every_catalog_variable_survives_ambient_options` is Claudine-scoped | it is `darkmatter/lib/tests/ambient_ctx_capture.rs`; out of this fix's AC5 boundary |
| five `lib/benches` entry points | one. `claude_parse.rs`, `opencode_parse.rs`, `pre_flight_checks.rs` and `prompt_preparation.rs` are **zero-byte tracked files** |

The plan's own instruction — "the guard's own count is authoritative and must
be re-read in Phase 5" — is why the spawn census was re-read from the artifact
rather than copied. It is stored at
`enumeration/recipes/spawn-site-burn-down.jsonl`.

### Findings

Four are new and none was found by looking at timing.

1. **Two dead runner overrides.** Both profiles carry
   `test(=composition::loop_engine::tests::seeded_loop_repro_runs_to_completion_with_live_derived_variable)`.
   The real identity is
   `composition::looping::engine::tests::seed_state::seeded_loop_repro_runs_to_completion_with_live_derived_variable`
   — the module was renamed and a `seed_state` module inserted. `test(=…)` is
   exact-match and nextest does not warn on a filter that selects nothing, so
   neither entry has bound to anything since the rename. The test runs in
   0.03 s under the ordinary ceiling.
2. **Four identities are unreachable** — they compile and run in no canonical
   recipe and no CI leg:
   `claudine-cli::wrap_sigint::slow_compose_sigint_during_prep_exits_130_with_notice`
   (the only `slow_` test in the eight packages; `l1-include-slow` is declared
   by darkmatter's four packages and by nobody else),
   `claudine-gen::signals_validation::real_corpus_builds_deterministically`,
   and the two `rendezvous-daemon::peer_discovery::real_mdns_*` cases (the
   rendezvous justfile has no `test-real` recipe at all). Nine further
   identities are `#[ignore]`d performance harnesses reachable only by
   `--ignored`, which no recipe passes.
3. **`rz-daemon-unit` is the largest hidden cost in the area**: 107.75 s summed
   over 153 identities (704 ms mean, the highest unit-test mean anywhere in the
   eight packages), in a crate the parent area's `just test` never runs. Its
   9.2 s elapsed hides it behind parallelism. This is a direct answer to the
   spec's first draft decision and Phase 3 inherits it.
4. **Four zero-byte bench files** are tracked and auto-discovered by cargo as
   empty bench targets.

Two more that confirm existing plan expectations with numbers:
`cli-l1-source-scan-guards` costs 39.53 s over 79 identities with `error_guards`
alone at 34.01 s over 18 cases — each case re-parsing the corpus in its own
process, which is exactly the consolidation candidate the spec asks about; and
`cli-l1-pty` is the most expensive L1 family per test (1.74 s mean over 11
identities).

### `inventory-reconciler.ts`

A gate, not a report. It rebuilds the runner universe from the captures,
assigns every identity to exactly one declared family, cross-checks
`inventory.md` against `families.json` and against the captures, scans the
source tree independently, and diffs the two populations.

Failure classes, each with named tests:

| Class | Detected as |
|---|---|
| malformed capture | not JSON, no `rust-suites`, a suite with no `testcases`, two suites sharing a `binary-id`, a JSON file in the directory the manifest does not declare |
| missing capture | a declared capture with no file |
| unassigned identity | an identity matched by no family |
| double-assigned identity | an identity matched by two — and it is counted by *neither*, so an overlap cannot inflate a total |
| stale family | a family matching nothing without an `expectEmpty` reason |
| inventory drift | a family with no row, a row naming no family, or a row whose count disagrees with the captures |
| missing disposition | an empty disposition, or one outside satisfactory / remediation in this fix / follow-up |
| undeclared exclusion | a source test the runner never lists that no exclusion explains |
| stale exclusion | a declared exclusion whose test now runs |

Exit 0 clean, 1 on violations, 2 on usage. Two design choices worth recording:

- **The source scan blanks string literals, char literals and comments before
  reading structure.** Without it, the raw-string fixture inside
  `cli/tests/test_placement.rs` — which exists precisely to test an analyzer's
  literal handling — is counted as a real test. It also resolves the area's one
  `macro_rules!` test template (`wrap_compose_agent.rs`'s
  `direct_wrap_dry_run_test!`) to its seven invocation sites, so the runner's
  seven macro-generated identities have source-side counterparts.
- **The source↔runner diff matches on `(package, leaf name)` multisets**, not
  full module paths: nextest reports `a::b::tests::name` while a text scan sees
  only `name`, and reconstructing the module path from text would guess.
  Counting per name keeps a `cfg(unix)`/`cfg(windows)` pair visible — source
  has two, the runner one. The limitation is stated in the code.

#### Verification

70 tests, `npx tsx --test inventory-reconciler.test.ts` — 70 passed, 0 failed.
`node:test` rather than Vitest, matching `junit-metrics.test.ts`.

Real-artifact coverage: five tests read the shipped captures, families and
inventory rather than fixtures —
`every_shipped_capture_parses_and_declares_a_package_kind_and_binary_id`,
`every_shipped_capture_is_declared_in_the_manifest_and_vice_versa`,
`the_shipped_families_assign_every_shipped_identity_exactly_once`,
`every_source_only_test_in_the_real_tree_is_a_declared_exclusion`, and
`the_real_tree_has_no_runner_identity_without_a_source_definition`. Two more
drive the whole gate end to end through `main` on the real tree
(`the_gate_passes_on_the_shipped_inventory_and_prints_its_table`,
`repeated_runs_over_one_tree_produce_an_identical_table`).

**Non-vacuity, by neuter.** Five detectors were disabled one at a time, the
suite re-run, and the file restored between each.

| Neuter | Named failures | Result |
|---|---|---|
| `blankLiteralsAndComments` returns its input | `a_test_module_quoted_inside_a_raw_string_is_a_fixture_not_a_test` + 11 lexer/scan tests | 58 pass / 12 fail |
| macro-invocation matching disabled | `a_macro_rules_test_template_yields_identities_at_its_invocation_sites`, `the_real_tree_has_no_runner_identity_without_a_source_definition` | 68 pass / 2 fail |
| module/test narrowing ignored (suite match wins) | `a_suite_plus_module_family_claims_only_that_module`, `a_module_prefix_stops_at_a_path_separator`, `an_exclude_block_removes_a_carve_out_from_its_parent`, `an_exact_test_entry_matches_the_full_identity_name`, `an_identity_matched_by_two_families_is_a_violation_and_is_counted_by_neither`, `the_shipped_families_assign_every_shipped_identity_exactly_once`, `the_gate_passes_on_the_shipped_inventory_and_prints_its_table`, `a_neutered_family_declaration_is_caught_rather_than_silently_reducing_a_count` | 62 pass / 8 fail |
| inventory count check disabled | `an_inventory_count_that_disagrees_with_the_captures_is_drift` | 69 pass / 1 fail |
| unassigned-identity detection disabled | `an_identity_matched_by_no_family_is_a_violation`, `a_neutered_family_declaration_is_caught_rather_than_silently_reducing_a_count` | 68 pass / 2 fail |

After restoring, `sha256(inventory-reconciler.ts)` is
`c72599020fe2bcfdb0ee82d7dfe99a7107b54c194fa54591a5ccc6ca9045da21`, identical
to the pre-neuter hash, and the suite is 70/70 again.

#### Requirement-to-test mapping

| Requirement clause | Tests |
|---|---|
| reads the nextest listings | `every_shipped_capture_parses_and_declares_a_package_kind_and_binary_id`, `a_union_across_captures_counts_each_identity_once`, `records_a_build_target_that_lists_no_test_instead_of_dropping_it` |
| reads the inventory's declared families | `parseFamilyFile` tests, `reads_the_family_index_regardless_of_column_order`, `ignores_tables_that_are_not_the_family_index`, `the_family_index_in_the_shipped_inventory_covers_every_declared_family` |
| **fails on an identity assigned to zero rows** | `an_identity_matched_by_no_family_is_a_violation`, `a_neutered_family_declaration_is_caught_rather_than_silently_reducing_a_count` |
| **fails on an identity assigned to more than one row** | `an_identity_matched_by_two_families_is_a_violation_and_is_counted_by_neither` |
| family matching is exact and bounded | `a_suite_only_family_claims_the_whole_suite`, `a_suite_plus_module_family_claims_only_that_module`, `a_module_prefix_stops_at_a_path_separator`, `a_family_never_reaches_another_package`, `an_exclude_block_removes_a_carve_out_from_its_parent`, `an_exact_test_entry_matches_the_full_identity_name` |
| stale families are caught | `a_family_matching_nothing_is_stale_unless_it_declares_why` |
| the inventory cannot drift from the captures | `a_family_with_no_inventory_row_is_drift`, `an_inventory_row_naming_no_family_is_drift`, `an_inventory_count_that_disagrees_with_the_captures_is_drift` |
| every row has a disposition | `an_empty_or_unrecognized_disposition_fails` |
| the source-side scan finds every attributed form | `finds_every_attributed_test_form`, `records_an_ignore_attribute`, `records_a_cfg_on_the_function_and_keeps_its_string_literal`, `inherits_a_cfg_from_the_enclosing_module`, `cfg_test_is_not_reported_as_a_platform_gate`, `an_attribute_separated_from_its_function_by_a_statement_does_not_bind` |
| …and no unattributed form (representation variants) | `a_test_module_quoted_inside_a_raw_string_is_a_fixture_not_a_test`, `a_macro_that_generates_no_test_contributes_no_identity`, `the_macro_template_itself_is_never_an_identity`, `a_lifetime_is_not_a_char_literal`, `a_char_literal_holding_a_brace_is_blanked_so_brace_counting_survives`, `a_nested_block_comment_is_blanked_to_its_true_end`, `a_raw_string_with_hashes_is_blanked_whole`, `a_byte_raw_string_is_blanked_like_a_raw_string`, `an_escaped_quote_does_not_end_a_string` |
| malformed source is rejected, not guessed at | `rejects_an_unterminated_block_comment`, `rejects_an_unterminated_string_literal`, `rejects_an_unterminated_raw_string` |
| cfg/feature exclusions are declared and stay honest | `a_source_test_the_runner_never_lists_is_source_only`, `a_per_platform_pair_of_the_same_name_shows_one_source_only_copy`, `an_undeclared_exclusion_fails_the_gate`, `a_declared_exclusion_that_now_runs_is_stale`, `a_declared_exclusion_that_still_holds_passes`, `every_source_only_test_in_the_real_tree_is_a_declared_exclusion` |
| malformed manifests and captures are rejected | `rejects_a_manifest_that_is_not_json`, `rejects_a_manifest_with_no_captures`, `rejects_a_manifest_missing_provenance_fields`, `rejects_a_capture_that_is_not_a_nextest_listing`, `rejects_a_capture_that_is_not_json`, `rejects_a_suite_with_no_testcases_object`, `rejects_one_listing_that_names_an_identity_twice`, `rejects_a_declared_capture_whose_file_is_missing`, `rejects_a_capture_file_that_no_manifest_record_declares`, `rejects_a_family_file_with_duplicate_ids`, `rejects_a_family_with_an_empty_match_block`, `rejects_a_family_row_with_a_non_numeric_identity_count` |
| exit codes: a printed miss is not a gate | `main_exits_two_without_an_enumeration_directory`, `main_exits_two_when_families_or_inventory_are_missing`, `main_exits_two_when_a_named_path_does_not_exist`, `main_exits_two_when_the_enumeration_path_is_a_file`, `main_exits_one_on_a_malformed_capture_rather_than_throwing`, `main_exits_one_when_a_family_row_is_missing_from_the_inventory` |
| passive corpus over shipped artifacts | `every_shipped_capture_parses_and_declares_a_package_kind_and_binary_id` (all 16), `every_shipped_capture_is_declared_in_the_manifest_and_vice_versa` |
| end to end via the real artifacts and the normal invocation path | `the_gate_passes_on_the_shipped_inventory_and_prints_its_table`, `repeated_runs_over_one_tree_produce_an_identical_table` |

There is no Rust behavior in this phase and therefore no Rust regression test.
Every behavior Phase 2 introduces lives in the reconciler, and every clause of
its requirement maps to a named test above.

### Gates

Run after the deliverables, from the working tree at `9fc5151a0` plus this
fix's documents.

| Gate | Area | Result | Wall |
|---|---|---|---|
| `npx tsx --test inventory-reconciler.test.ts` | fix directory | 70 passed, 0 failed | 3.8 s |
| `inventory-reconciler.ts …` | fix directory | `GATE EXIT=0` | 1.2 s |
| `just test` | `claudine` | 6861 passed, 11 skipped, **exit 0** | 36.2 s |
| `just lint` | `claudine` | **exit 0** | 6.5 s |
| `just test` | `sniff` | 2599 passed, 23 skipped, **exit 0** | 30.6 s |
| `just lint` | `sniff` | **exit 0** | 48.5 s |
| `just test-rendezvous` | `claudine` | 82 + 169 + 21 passed, 2 skipped, **exit 0** | 13.9 s |
| `just doctest` | `claudine` | 32 doctests, **exit 0** | 6.0 s |

`sniff` is included because the session was started in that area; no `sniff`
file was read or written by this phase, and its gates are a confirmation that
nothing regressed, not a claim of coverage.

`git diff main -- .config/nextest.toml` remains empty (AC5, AC7).

### Deliberate choices

- **The captures are committed uncompressed** (3.4 MB across 16 files). They
  are the evidence a reviewer re-runs the gate against, and the gate's own test
  suite reads them as a passive corpus; a compressed substrate would need a
  decompression step in both paths for no reviewable gain.
- **Families are declared, not derived.** A family computed from the source's
  own properties would make the reconciler agree with itself. Declaring
  suites/modules and proving the declaration total is the only version of this
  gate that can fail.
- **Fifty families, not 163 (one per build target) and not eight (one per
  package).** The axis is the execution contract — what a test costs and what
  it can contaminate — because that is what a test-performance inventory is
  for. Where a member's setup or proof differs from its neighbours it was
  carved out; eleven were.

### Validation checkpoint 2 — passed

| Requirement | Status |
|---|---|
| The reconciler exits 0 | **done** |
| Inventory covers all eight packages | **done** — 7,400 identities, 163 build targets |
| …all tiers | **done** — L1/L2/L3/real; L3 and real named *pending* |
| …doctests, benches, excluded/ignored tests, shared fixtures | **done** |
| Every row has a disposition | **done** — 26 satisfactory, 21 remediation, 3 follow-up |
| The override census is complete | **done** — 24 blocks, 11 Claudine-scoped with verdicts, 7 attributed elsewhere |

### Carried forward

- Phase 3 inherits the attribution questions with numbers already attached:
  `rz-daemon-unit` (107.75 s), `cli-l1-source-scan-guards` (39.53 s),
  `cli-l1-pty` (1.74 s/test), `cli-l2-lifecycle` (200.35 s).
- Phase 5 must re-read `spawn-site-burn-down.jsonl` rather than the plan's
  census table.
- Phase 6 inherits the two dead overrides, the four unreachable identities, the
  nine ignored perf harnesses, `test-real`'s `cargo test` route, rendezvous's
  missing recipes, and the five crates with no `[package.metadata.ci.tests]`.
- Phase 10 inherits the four zero-byte bench files and the `rust-testing`
  skill's missing cross-compilation route (Phase 1 finding 2). No skill file
  was changed in this phase.

## Phase 3 — Attribution and ratified budgets (RB5 first half)

**Outcome: complete for three of the spec's four draft decisions; the fourth —
budgets — is refused rather than answered, because its input is Phase 1's CI
baseline and that is still blocked.** Document-only: no Rust package source was
changed. Phases 4–10 remain blocked on Phase 1's checkpoint.

Deliverables: [`attribution.md`](attribution.md),
[`attribution.ts`](attribution.ts) with [55 tests](attribution.test.ts),
[`attribution/launch-cwd-probe.ts`](attribution/launch-cwd-probe.ts), the
evidence under [`attribution/`](attribution/), and the rewritten
[`inventory.md` § Budgets](inventory.md#budgets).

### Grounding facts re-checked

Two of the plan's facts moved, both checked in the tree rather than assumed.

| Plan said | Measured at `9fc5151a0` |
|---|---|
| the live-child cohort is six files (`wrap_sigint`, `handle_deadline`, `compose_ttff_perf`, the two Windows files, `spawn_inventory`) | **five.** `spawn_inventory.rs`'s `.spawn()` sites are inside a raw-string fixture its own scanner test parses; it owns no child |
| Phase 5D routes `common/pty.rs`'s session construction | `common/pty.rs` owns draining and marker helpers only. The commands are built in the three PTY binaries (`compose_command`, `sequence_command`, `sequence_command_no_provider`, plus two inline `Command::new(cargo_bin(…))` sites) |

The plan's link to the startup-stall spec is also stale — that fix now lives
under `fixes/_completed/`. `attribution.md` uses the corrected path; Phase 10
should fix the plan's own link.

### The finding that reframes the phase

`.config/nextest.toml`'s CI profile binds `claudine-cli`'s entire L1 suite to
`test-group = 'claudine-cli-ci-l1'`, which is `max-threads = 1`, and
`claudine`'s to `claudine-l1` at 4. **On CI those suites are serial or
near-serial, so summed test duration is the leg's floor rather than a number
parallelism hides.** Locally the same suite reports 289.12 s summed behind
18.12 s elapsed. Every attribution below is therefore stated in summed
duration, and the plan's instruction that a local run may not set a target
gains a second, independent reason: local numbers are inflated by contention
(`error_guards` sums 20.23 s isolated and 34.01 s in-suite) *and* deflated by
parallelism, and neither correction is arithmetic.

### Answers

1. **Which non-spawn families account for the remaining cost** — the L2
   capture trio (443.08 s), the library's `composition::sequence` and
   `composition::schema` modules inside `lib-unit` + `lib-unit-task-shell`
   (254.72 s), `rz-daemon-unit` (107.75 s), `cli-l1-source-scan-guards`
   (39.53 s) and `context_command` (27.87 s). The 1,694-identity unit block —
   the largest count in the area — costs 49.68 s at a 0.029 s mean. Count is
   not where the cost is.
2. **Which source scans can share work** — exactly one: the twelve corpus
   cases inside `error_guards`, which each repeat a ~1.7 s `syn` parse whose
   `OnceLock` cache is process-local. The counterfactual is measured, not
   assumed: the same eighteen assertions cost **1.61 s** in a single process
   under `cargo test` against 20.23 s under nextest — ~18.6 s of recoverable
   repeated work. The five text scanners together hide ~1.2 s and must stay
   separate; cross-binary sharing is rejected on the same evidence.
3. **Which technical exceptions remain necessary** — none.
   `expectrl::Session::spawn` takes a `std::process::Command` by value, and
   every other live-child need (signal delivery, `CREATE_NEW_PROCESS_GROUP`,
   streaming stdout, reaping) is a call-site concern. `NEEDS_LIVE_CHILD` can
   close in Phase 5D.
4. **What budgets are justified** — unanswerable today, and refused rather
   than guessed. See below.

### The launch-CWD measurement

`context_command.rs` launches the real binary with `current_dir(repo_root())`
in 22 of its 27 cases. The probe measures what that alone costs:

| CWD | 1 | 8 | 16 | 27 concurrent |
|---|---:|---:|---:|---:|
| monorepo root | 672 ms | 1,953 ms | 3,812 ms | 6,040 ms |
| outside a repository | 20 ms | 24 ms | 36 ms | 52 ms |

34× per process, 116× at the concurrency nextest actually uses. Two competing
explanations were measured and cleared: binary startup is noise (27 concurrent
launches outside a repository total 52 ms), and a checkout-rooted spawn is not
automatically expensive (`errors_command.rs` does the same thing in all five
cases for 0.50 s summed, because `claudine errors` captures no repository
context). The tax is paid by commands that capture repository context.

### Budgets: a refusal, not a number

No budget was written. `deriveBudgets` refuses non-CI provenance before reading
a single measurement, and refuses a leg with fewer than three consecutive green
runs, a declared leg with no measurements, and a family with fewer samples than
runs. `attribution/budgets-pending.json` declares the four legs at zero runs so
the refusal is reproducible today. The *procedure* — worst-of-three per leg,
×1.25 headroom, legs never merged, misses reported rather than absorbed — is
fixed now, before the numbers exist, which is the only point at which fixing it
means anything.

### `attribution.ts`

A gate, not a report. It parses nextest run logs, joins each result to a family
through the reconciler's own `familiesMatching`, and totals summed/mean/max per
family and per binary. Failure classes, each with named tests:

| Class | Detected as |
|---|---|
| truncated log | no summary line at all; result lines after the last summary |
| not a nextest run | a lint or build log attributed as zero cost instead of rejected |
| malformed duration | a bracket that is not a number |
| count mismatch | decided result lines disagreeing with the run's own summary |
| red run | any non-`PASS` terminal status; a red run measures nothing |
| duplicate identity | the same identity twice at the same attempt (retries collapse) |
| unassigned / double-assigned identity | zero or two families claiming a result; it is counted by neither |
| local-derived budget | budget input whose provenance is not CI |
| insufficient runs / missing leg | fewer than three green runs, or a declared leg with no measurements |

#### Verification

55 tests, `npx tsx --test attribution.test.ts` — 55 passed, 0 failed.
`node:test`, matching `junit-metrics.test.ts` and `inventory-reconciler.test.ts`.

Real-artifact coverage: three tests read the shipped evidence rather than
fixtures — `every_shipped_nextest_log_parses_and_reports_a_complete_green_run`
(every log under `baseline/local-gates/`, `enumeration/recipes/` and
`attribution/runs/`), `every_result_in_every_shipped_log_belongs_to_exactly_one_declared_family`
(7,000+ results against the real `families.json`), and
`the_whole_gate_runs_end_to_end_over_the_real_local_gate_log_and_the_real_families`.
A fourth reads one log twice and asserts an identical table.

**Non-vacuity, by neuter.** Five detectors were disabled one at a time, the
suite re-run, and the file restored between each.

| Neuter | Named failures | Result |
|---|---|---|
| budget provenance check disabled | `a_budget_is_never_derived_from_local_evidence`, `main_exits_one_and_prints_no_budget_table_when_the_budget_input_is_local` | 51 pass / 2 fail |
| failed-run detection disabled | `a_red_run_measures_nothing_and_is_rejected` | 52 pass / 1 fail |
| count-mismatch detection disabled | `a_run_whose_result_lines_disagree_with_its_own_summary_count_is_a_violation` | 52 pass / 1 fail |
| double-assignment detection disabled | `an_identity_two_families_claim_is_a_violation_and_is_counted_by_neither` | 52 pass / 1 fail |
| truncated-log rejection disabled | `a_log_that_is_not_a_nextest_run_at_all_is_rejected_not_attributed_as_zero_cost`, `main_exits_one_when_handed_a_log_that_is_not_a_nextest_run` | 53 pass / 2 fail |

**The last row is a test the neuter forced into existence.** On the first pass
that neuter produced 53 pass / 0 fail: a truncated log with result lines still
tripped the *second* guard, so no test distinguished the two. A log with
neither results nor a summary — a lint or build log — would have been reported
as an empty table with `GATE EXIT=0`, which is precisely the "prints a miss and
exits 0" failure the plan forbids. Two tests were added for that input, and the
neuter now fails as it should.

After restoring, `sha256(attribution.ts)` is
`8593be97b676f4ec72900d4a33929bcdbf4e8b6d0f17deeb08afb639519d9d57`, identical
to the pre-neuter hash, and the suite is 55/55 again.

#### Requirement-to-test mapping

| Phase 3 requirement | Tests |
|---|---|
| attribute cost by family against the baseline | `every_result_lands_in_exactly_one_family_and_the_totals_are_its_own`, `families_are_ranked_by_summed_cost_not_by_name_or_count`, `shares_are_fractions_of_the_attributed_total_and_sum_to_one`, `attribution_accumulates_across_several_runs_of_the_same_family`, `a_module_scoped_family_claims_only_its_module_as_in_the_reconciler` |
| keep build, elapsed and summed apart | `the_rendered_table_carries_the_three_costs_and_never_invents_a_fourth`; the build column comes from `attribution/isolated-runs.tsv`, whose `wall − elapsed` split is the recipe's own |
| measure source-scan families individually, with process counts | `per_binary_attribution_counts_one_process_per_result_and_keeps_the_cheapest_case`, `per_binary_attribution_needs_no_families_and_so_cannot_be_skewed_by_one`, `a_retried_test_is_one_process_in_the_per_binary_table_not_two`, `the_binary_table_renders_the_process_count_beside_the_summed_cost`, `main_renders_the_binary_table_when_asked_and_the_family_table_otherwise` |
| **no budget from a local run alone** | `a_budget_is_never_derived_from_local_evidence`, `main_exits_one_and_prints_no_budget_table_when_the_budget_input_is_local` |
| three consecutive green runs per leg | `a_leg_with_fewer_than_three_consecutive_green_runs_yields_no_budget`, `a_family_with_fewer_samples_than_runs_yields_no_budget_for_the_whole_set` |
| legs are never merged | `every_leg_gets_its_own_budget_cross_platform_counts_are_never_merged`, `a_declared_leg_with_no_measurements_is_a_missing_leg_not_an_average_of_the_others`, `no_leg_at_all_is_a_violation_rather_than_an_empty_pass` |
| the budget formula is one declared constant | `a_complete_ci_set_yields_a_budget_from_the_worst_run_plus_headroom`, `headroom_is_explicit_and_changes_the_ceiling_not_the_observed_maximum`, `a_negative_or_non_finite_headroom_is_rejected_before_any_budget_is_computed`, `the_budget_table_renders_one_row_per_leg_and_family` |
| evidence must be complete and green | `a_log_with_no_summary_is_truncated_evidence_and_is_rejected`, `a_log_that_is_not_a_nextest_run_at_all_is_rejected_not_attributed_as_zero_cost`, `result_lines_after_the_last_summary_are_rejected_as_a_truncated_tail`, `a_non_numeric_duration_is_rejected_rather_than_silently_costing_zero`, `a_run_whose_result_lines_disagree_with_its_own_summary_count_is_a_violation`, `a_red_run_measures_nothing_and_is_rejected`, `the_same_identity_reported_twice_at_the_same_attempt_is_a_duplicate`, `a_retried_test_counts_once_and_its_final_attempt_decides_the_cost` |
| representation variants of the runner's own output | `strips_the_sgr_sequences_nextest_writes_even_into_a_redirected_file`, `a_duration_in_brackets_survives_ansi_stripping`, `a_summary_with_skipped_and_slow_counts_still_parses`, `a_slow_notice_is_informational_and_never_becomes_a_result`, `one_log_holding_several_nextest_invocations_yields_one_run_per_summary`, `leak_timeout_and_abort_statuses_parse_as_terminal_results`, `a_retry_line_records_its_attempt_number`, `package_is_the_segment_before_the_first_double_colon_and_a_bare_suite_is_its_own_package` |
| exit codes: a printed miss is not a gate | `main_exits_two_without_a_log_or_a_families_file`, `main_exits_two_when_a_named_path_does_not_exist`, `main_exits_two_when_top_is_not_a_non_negative_integer`, `main_exits_one_on_a_truncated_log_rather_than_throwing`, `main_exits_one_when_a_result_belongs_to_no_family`, `main_exits_zero_and_prints_the_table_on_a_clean_log`, `a_non_json_budget_file_is_a_usage_error_not_a_violation` |
| passive corpus over shipped artifacts | `every_shipped_nextest_log_parses_and_reports_a_complete_green_run`, `every_result_in_every_shipped_log_belongs_to_exactly_one_declared_family` |
| end to end through the normal invocation path | `the_whole_gate_runs_end_to_end_over_the_real_local_gate_log_and_the_real_families`, `the_shipped_local_gate_logs_attribute_the_same_summed_cost_every_time_they_are_read` |

Phase 3 changes no Rust behavior and therefore adds no Rust regression test.
Every behavior it introduces lives in `attribution.ts` and maps to a named test
above. `launch-cwd-probe.ts` is an evidence generator rather than a gate; its
only correctness claim — that every child it timed exited 0 — is enforced by
its own exit code (it returns 1 and writes no table if any child fails).

### Measurement method

- **Isolated per-binary runs**: `just test-cli --test <binary>`, three
  repetitions, wall time recorded outside the recipe, nextest's elapsed and the
  summed per-test durations parsed from the log
  (`attribution/isolated-runs.tsv`, logs in `attribution/runs/`). The recipe's
  own setup floor is ~1.2 s and is visible as `wall − elapsed` on the trivial
  binaries.
- **Shared-process counterfactual**: `cargo test -q -p claudine-cli --test
  <binary>`, three repetitions (`attribution/shared-process-runs.tsv`). This is
  a measurement of what a shared scan costs, not a proposal to run the suite
  that way — `cargo test` forfeits per-test isolation, the leak policy and
  selective execution.
- **Launch-CWD probe**: `attribution/launch-cwd-probe.ts`, one warm-up burst per
  location, then 1/8/16/27 concurrent invocations from each
  (`attribution/launch-cwd.tsv`).
- **Full-suite attribution**: Phase 1's and Phase 2's recorded gate logs, read
  through `attribution.ts`. No suite was re-run for attribution, so the numbers
  are the same evidence Phase 2 dispositioned against.

`just test-cli -E '…'` cannot carry a filterset through this area's recipes:
`test-cli` interpolates its arguments into a second `just _test` invocation,
which expands them into a bash array (`forwarded=({{ args }})`), so the
parentheses in `binary(x)` are a syntax error two layers down. The general
answer is `BISCUIT_TEST_FILTER`, which `_tier_filter` honors as a total
override of the tier expression — at the cost of re-including the L1 exclusions
by hand. For a *single* binary, cargo's own `--test <stem>` passes through
cleanly and keeps the recipe's tier filterset intact, which is why every
isolated run above uses it.

### Gates

Run after the deliverables, from the working tree at `9fc5151a0` plus this
fix's documents. Tests before lint, per Phase 1's stale-binary note.

| Gate | Area | Result | Wall |
|---|---|---|---|
| `npx tsx --test attribution.test.ts` | fix directory | 55 passed, 0 failed | 0.2 s |
| `attribution.ts …` (family, binary, budget modes) | fix directory | `GATE EXIT=0`, `GATE EXIT=0`, `GATE EXIT=1` (the intended refusal) | 1–3 s |
| `npx tsx --test inventory-reconciler.test.ts` | fix directory | 70 passed, 0 failed | 3.6 s |
| `inventory-reconciler.ts …` | fix directory | `GATE EXIT=0` after the `inventory.md` edits | 1.2 s |
| `just test` | `claudine` | 6861 passed, 11 skipped, **exit 0** | 35.8 s |
| `just lint` | `claudine` | **exit 0** | 77.2 s |
| `just test` | `sniff` | 2599 passed, 23 skipped, **exit 0** | 32.2 s |
| `just lint` | `sniff` | **exit 0** | 13.0 s |

`sniff` is included because the session was started in that area; no `sniff`
file was read or written by this phase.

`git diff main -- .config/nextest.toml` remains empty (AC5, AC7).

### Deliberate choices

- **The budget refusal is code, not a promise.** A sentence saying "budgets are
  pending" survives exactly as long as the next person's patience. A gate that
  exits 1 on local provenance survives contact with Phase 9.
- **The ratification procedure is fixed before its input exists.** Choosing
  worst-of-three and a single ×1.25 constant now removes the opportunity to
  choose a statistic that flatters the candidate run later.
- **No suite was re-run to attribute the full suite.** Re-running would have
  produced numbers that no longer matched the inventory Phase 2 dispositioned
  against; the point of the gate is that the two documents read the same
  evidence.
- **No production defect was filed.** The launch-CWD asymmetry is the only
  candidate and it does not meet the bar: the expensive side renders 733 rows
  of real repository content the cheap side never produces. Recorded in
  `attribution.md` with its evidence so a later phase can reopen it with a work
  counter.

### Validation checkpoint 3 — partially passed

| Requirement | Status |
|---|---|
| Every draft decision has a written answer backed by a measurement | **3 of 4.** Decision 4's answer is a tested refusal plus a fixed procedure; its input is blocked on Phase 1 |
| Budgets are in `inventory.md` beside the baseline | **procedure and refusal are; the numbers are pending** |
| No budget was derived from a local run alone | **done** — none exists, and the gate refuses local provenance by construction |

### Carried forward

- Phase 5C's payoff on `context_command` is 34–116× per launch, not a marginal
  cleanup; Phase 8 should prove it with a work counter, not only with timing.
- Phase 5D must name the three PTY binaries' command builders rather than
  `common/pty.rs`, and may treat `spawn_inventory.rs` as out of the live-child
  cohort entirely.
- Phase 6 inherits one consolidation candidate with a measured payoff
  (`error_guards`, ~18.6 s) and four rejections with the numbers behind them.
- Phase 6/7 must not treat the watchdog, loop-pause and handle-deadline floors
  as reduction targets; they are timeout contracts under the startup-stall
  fix's spawn-fallback clock, and each already parameterises its budget.
- Phase 9 fills `attribution/budgets-pending.json`'s `perLegFamilySummed` and
  raises `runsPerLeg` to 3; nothing else in the budget path needs to change.
- Phase 10 should correct the plan's stale link to the startup-stall spec.
- A `rust-testing` skill candidate for Phase 10: narrowing an area recipe to
  one test binary. `-E 'binary(x)'` cannot survive the two-layer argument
  interpolation; `BISCUIT_TEST_FILTER` is the documented total override, and
  `--test <stem>` is the cheaper spelling when one binary is all that is
  wanted, because it leaves the recipe's tier filterset alone. Recorded here
  rather than edited into the skill, because this phase's scope is
  document-only and the plan puts skill updates in Phase 10.

---

## Phase 4 — One environment policy, two command surfaces (RB2 infrastructure)

**Outcome: complete.** The L1 spawn contract is now computed once as data and
applied to both `assert_cmd::Command` and `std::process::Command`, so the
live-child cohort Phase 5D migrates has a fixture-built command to migrate
*to*. The area's gates are green except one host-condition L2 failure,
disclosed below. Nothing was staged or committed: Phase 1's checkpoint is still
blocked on the operator merge, and the plan forbids a Phase 4–10 commit landing
first.

### Grounding facts re-checked

Both of the plan's Phase 4 census numbers are stale, and Phase 5 should read
the guard rather than the plan:

| Fact as planned | As the guard reads it today |
|---|---|
| 170 spawn sites in 36 allow-listed files, 83 governed | **172 sites**, 36 files, **89 governed** |
| isolation gate governs 32 files | **37 files**, still **0 escapes**, `ISOLATION_ALLOWLIST` still empty |

The drift is not this phase's: no `.rs` file was added to `cli/tests` here
except `common/host_tools.rs`, which the `common/` exclusion keeps out of both
populations. The counts were taken before Phase 1's local-gate work landed.

Two facts held exactly as written: the environment policy was applied inline in
`ClaudineCommandBuilder::build` against `assert_cmd::Command` receivers, and
`assert_cmd::Command` still offers no `spawn` and no route back to the inner
`std::process::Command`.

### What shipped

**`common/mod.rs` — the policy as data.** `ChildEnvironment` carries a clear
flag, an ordered `Vec<EnvironmentOp>` of removes and sets, and the pinned
`current_dir`. `ChildEnvironment::apply` is the only code that decides what a
`claudine` child inherits; the two `ConfigurableCommand` impls are four
one-line methods each. Ordering is preserved by construction — the ops are a
sequence, not a map, because two of the contract's rules *are* ordering rules
(`CLAUDINE_RENDEZVOUS_REPORT` set after the `CLAUDINE_*` sweep; the Windows
console restore after the clear).

Two by-products worth naming:

- `scrub_inherited_environment(&mut Command)` became
  `inherited_scrub_keys() -> Vec<OsString>`, and
  `restore_windows_console_variables(&mut Command)` became
  `windows_console_variables() -> Vec<(OsString, OsString)>`. The latter now
  tests the platform with `cfg!(windows)` instead of `#[cfg(windows)]`, so the
  Windows arm **compiles on every leg** rather than only on `windows-latest` —
  a typo in it used to be a Windows-only failure.
- `build()` keeps `assert_cmd::Command::cargo_bin("claudine")`; `build_std()`
  resolves through `claudine_bin()` (`biscuit_test_harness::bin_exe!`), which
  is strictly more robust — it honors nextest's run-time republication, so a
  relocated archive run on the `wsl2-ubuntu` leg finds the binary. `build()`
  was left alone deliberately: the plan says it keeps its behavior, and which
  file it resolves is part of that.

**`common/host_tools.rs` — the parent-side half.** `GIT_PLUMBING_VARS` and the
new `helper_command(program)` moved into their own file so a binary can include
just them with `#[path]`, the way `spawn_site_guard.rs` includes
`common/source_scan.rs`. `system_prompt_perf_bench.rs` uses exactly that: it
shells out to `git` and never spawns `claudine`, so pulling in the whole
fixture surface for one call would have been the wrong trade.

**The helper-command audit** (plan bullet 8), in full:

| Site | Verdict |
|---|---|
| `common/mod.rs::init_git_repo` | now `helper_command("git")` |
| `sequence_magic_reference.rs` ×3, `loop_cli.rs`, `level2_lifecycle_control.rs` | now `common::helper_command("git")` |
| `system_prompt_perf_bench.rs` | now `host_tools::helper_command("git")` via `#[path]` |
| `rustc` ×4 (`compose_caller_file_provenance`, `sequence_cli`, `inline_compose_hash`, `wrap_ctrl_c_windows`) | left alone — `rustc` consults no `GIT_*` variable, and it compiles a stub rather than reading repository state |
| parent-side `md` | none exists in the L1 suite; `inline_compose_hash.rs` calls the darkmatter library directly, and the area's `_ensure-md` recipe builds the binary for the *child* |

`level2_lifecycle_control.rs` is outside both gates (L2), but its `git` is the
same door: an inherited `GIT_DIR` would relocate its baseline commit into the
real repository, which is the 2026-08-31 incident verbatim.

**The guards.** No detector arm changed behavior. The spawn gate already treats
`command_std()` / `build_std()` as sanctioned, because neither names a binary —
what was missing was the proof, and the module docs stating it. Three tests
were added: the sanctioned-form pair, the isolation forms on a raw command
(including the negatives that keeping a child is not an escape), and the one
that pins *why* no widening edit was needed —
`deleting_a_spawn_entry_is_what_widens_the_isolation_population`.

### Requirement → test map

| Phase 4 requirement | Test (file) |
|---|---|
| one policy, two surfaces, same effective environment | `both_command_surfaces_hand_the_child_the_same_environment` (`cli_process_fixture.rs`) |
| …including the cleared arm and the Windows console restore, via `cfg!` | `both_command_surfaces_clear_the_environment_the_same_way` |
| the raw surface can actually hold a child | every raw-path test: `run_probe_std` uses `spawn()` + `wait_with_output()`, not `output()` |
| `fake_only_path()` / `host_path()` on the raw path | `the_raw_command_surface_keeps_the_named_path_escapes` |
| `ambient_context(dir)` on the raw path | `the_raw_command_surface_keeps_the_ambient_context_escape` |
| …and its containment rejection | `the_raw_command_surface_rejects_an_ambient_context_outside_the_workspace` |
| `inherit_no_env()` on the raw path | `both_command_surfaces_clear_the_environment_the_same_way` |
| containment holds *after* canonicalization | `a_workspace_inside_the_checkout_is_rejected_by_naming_the_temp_dir_variable` (new uncanonical-spelling case) |
| parent-side helper tools cannot be relocated | `a_parent_side_git_cannot_be_relocated_by_an_inherited_gitdir` |
| the builder's raw path is sanctioned; a hand-rolled one is not | `detector_treats_the_builders_raw_command_path_as_a_sanctioned_form` (`spawn_site_guard.rs`) |
| the isolation gate reads a raw command like an `assert_cmd` one | `isolation_detector_reads_a_raw_fixture_command_like_an_assert_cmd_one` |
| the isolation population widens by deleting a spawn entry | `deleting_a_spawn_entry_is_what_widens_the_isolation_population` |

Nine tests added, none removed: `just test-cli` 2489 → 2498, `just test`
6861 → 6870. Additions and removals are reported separately and both reconcile
to +9 / −0.

Representation variants covered, rather than only the happy path: native vs.
cleared environment (`inherit_no_env`), present vs. absent inherited values
(the `CONTROL` sentinel proves the parent reaches the child, so every `[]` is a
removal and not a passthrough that never happened), all three `PATH` policies,
the pinned vs. escaped launch CWD, and the two error cases the escape rejects
(a directory outside the workspace, a directory that does not exist).

### Non-vacuity transcripts

Each neuter was applied to the working tree, the named test observed failing
through `just test-cli --test <binary>`, then the file restored from a byte copy
and the SHA-1 compared back to the original. All seven restored identical.

| Neuter | Failure it produced |
|---|---|
| `build_std` drops the `Remove` ops | `both_command_surfaces_hand_the_child_the_same_environment` FAIL (19/20 passed) |
| `helper_command` keeps the `GIT_*` family | `a_parent_side_git_cannot_be_relocated_by_an_inherited_gitdir` FAIL |
| `windows_console_variables` drops its `cfg!` guard | `both_command_surfaces_clear_the_environment_the_same_way` **and** `inherit_no_env_keeps_the_defaults_and_drops_everything_else` FAIL |
| `child_environment` ignores the `PATH` policy | `the_raw_command_surface_keeps_the_named_path_escapes` + 3 pre-existing PATH tests FAIL |
| spawn detector loses the `claudine_bin` arm | `detector_treats_the_builders_raw_command_path_as_a_sanctioned_form`, `detector_finds_every_spawn_form_as_executable_code`, and the live gate `l1_tests_spawn_claudine_through_the_fixture_builder` FAIL |
| isolation detector loses the `env_clear` arm | `isolation_detector_reads_a_raw_fixture_command_like_an_assert_cmd_one` + the pre-existing detector test FAIL |
| `governs_isolation` ignores `spawn_allowlisted` | `deleting_a_spawn_entry_is_what_widens_the_isolation_population` FAIL |

The `git init` half was checked against the tool before it was asserted:
`GIT_DIR=$PWD/hijacked git -C work init` creates `hijacked/` and leaves
`work/.git` absent, which is exactly the failure the test now detects.

### Gates

Tests before lint, per Phase 1's stale-binary note. Run from the `claudine`
package area.

| Gate | Result | Wall |
|---|---|---|
| `just test-cli` | 2498 passed, 10 skipped, **exit 0** | 41 s |
| `just test` | 6870 passed (1 slow), 11 skipped, **exit 0** | 36 s |
| `just test-l2` | 236 passed, **1 failed**, 2512 skipped | 108 s |
| `just lint` | **exit 0**, no warnings | 75 s |
| `just check-windows` | **exit 0** (`x86_64-pc-windows-gnu`, `--tests`) | 4 s (warm) |
| `git diff main -- .config/nextest.toml` | empty | — |

**The L2 failure is a host condition, not a regression.**
`level2_typed_error_render_capture::level2_initialize_proxy_block_auto_detects_osc8_in_wezterm`
fails with "the `claudine_rc:<code>` exit marker did not appear within 30s",
and the captured frame shows Atuin's first-run prompt sitting in the spawned
WezTerm pane:

```text
plain:
  Atuin AI is not yet configured.
    Enable Atuin AI
```

The pane's interactive login shell is swallowing the `; echo claudine_rc:$?`
trailer. It reproduces when the binary is run alone, it is not in any code path
this phase touched, and the other 236 L2 tests — every one of which compiles
`common/mod.rs` — pass. Recorded as **failed, host condition**, not as passing.

`just check-windows` carries three pre-existing `unused_imports` warnings in
`wrap_basics.rs` and `compose_caller_file_provenance.rs`, byte-identical to the
ones in Phase 1's `baseline/local-gates/just-check-windows.log`. Nothing this
phase touched warns on either platform.

### Deliberate choices

- **The policy is a `Vec` of ops, not a map.** A map would have to re-encode
  the ordering rules as sort keys or comments; a sequence carries them.
- **The adapters are a trait with renamed methods** (`pin_current_dir`, not
  `current_dir`). An adapter that merely forwarded identical names invites the
  next reader to delete it and call the inherent method — which is how the
  policy would drift back into the call sites.
- **`cfg!` over `#[cfg]` for the Windows restore.** It costs nothing and turns
  a Windows-only failure mode into a compile-everywhere one; the Unix arm of
  the drift test then *asserts* the restore stays Windows-only.
- **No `ISOLATION_ALLOWLIST` entry was added.** The one place that wanted a
  `.current_dir` on a non-claudine command — the new parent-side `git` test —
  goes through `common::init_git_repo` instead, so the allow-list stays empty
  and the guard's claim that "every legitimate need is a named builder method"
  stays true.
- **`rustc` sites left alone.** The bullet asks for an audit for *inherited Git
  plumbing*; `rustc` reads none. Routing them through `helper_command` would
  have been churn dressed as hardening.

### Validation checkpoint 4 — passed, with one disclosed failure

| Requirement | Status |
|---|---|
| `just test-cli` green | **yes** — 2498 passed |
| `just lint` green | **yes** |
| `just test-l2` green | **236 of 237**; the survivor is the Atuin/WezTerm host condition above |
| both censuses printed and unchanged but for the sanctioned form | **yes** — 172 spawn sites / 36 files, 0 isolation escapes |
| `git diff main -- .config/nextest.toml` empty | **yes** |

### Carried forward

- **Phase 5 must re-read the guard, not the plan.** The per-file census in the
  plan's grounding facts is stale by two sites and six governed files.
- **Phase 5D's cohort now has its target**: `fixture.command_std()` /
  `command_builder()…build_std()`, with `.spawn()`, `.stdout(…)`, and
  `.creation_flags(…)` all left unflagged by the isolation gate. The three PTY
  binaries can hand `build_std()` straight to `expectrl`'s `Session::spawn`,
  which takes a `std::process::Command` by value.
- **`NEEDS_LIVE_CHILD` can close in 5D.** Nothing in the reason survives the
  raw path, as Phase 3's decision 3 predicted.
- **A `rust-testing` skill candidate for Phase 10**: a shared policy applied to
  two command types is a trait plus one `apply`, not a duplicated builder — and
  the drift test belongs against the recorded child environment, not against
  the builder's internals.
- **The L2 Atuin condition will recur on this host** until the prompt is
  answered outside the suite. Phase 8/9 should not read it as a candidate
  regression.

---

## Phase 5 — L1 spawn burn-down (RB2, AC2)

`SPAWN_ALLOWLIST` is empty. 36 files and 172 raw sites became zero, the
isolation gate's governed population went 37 → 74 files with zero escapes and an
`ISOLATION_ALLOWLIST` that is still empty, and `cli/tests/contamination_probes.rs`
is the AC3 evidence.

### The plan's batch lists had a hole

The four batches named 35 of the 36 allow-listed files.
`compose_interactive_timeout_cli.rs` (5 sites) appears in none of them, so 5E's
"zero generic exemptions" could not have been reached by following the plan
literally. It was migrated with the rest of the compose family in 5A.

### Findings the migration surfaced

These are behavior facts the burn-down exposed, not choices it made.

1. **Two L1 tests were passing only because of inherited state.**
   `level1_inline_compose_mismatch_pty.rs` read the developer's own
   `~/.claudine/config.json`; under a fixture home the first-run wizard
   intercepted the PTY and swallowed the transcript. It now seeds its own
   config. `compose_schema_cli`'s shipped-plan-route test launched from the
   rusty-biscuit checkout itself; it now copies the two shipped artifacts it
   exercises to their checkout-relative paths inside the fixture.

2. **Three provider stubs wrote to `$HOME` and were read back from the workspace
   root.** The same directory before the migration, two after it. Each now names
   the file it means — `sequence_jit`'s `::shell wc -l < log.txt` and its
   document-rewrite stub take the launch directory's path; the `launches.txt`
   witnesses read from `fixture.home()`.

3. **`GIT_DIR` / `GIT_WORK_TREE` do not move `ctx.repo_root`.** Verified
   directly: claudine's repository discovery walks the filesystem rather than
   honoring the plumbing variables, so a leaked pair does not relocate the
   *child's* repository. The scrub is still right — the predecessor's actual
   incident was a *parent-side* `git init` relocated by `GIT_DIR` — and that is
   what the Git probe exercises. `common/mod.rs`'s claim that the family
   "overrides cwd-based repository discovery and so defeats the pinned
   `current_dir`" is stronger than what this path does; it is left as written
   because it is true of any `git` subprocess, but a Phase 6 reader should know
   the child-side half is unproven.

4. **A single `compose` run consults no `CLAUDINE_*` variable observably.**
   `CLAUDINE_TIMEOUT=0.01s`, `CLAUDINE_STEP_TIMEOUT=0.01s`,
   `CLAUDINE_SYSTEM_PROMPT`, `CLAUDINE_FAIL_FAST` and `CLAUDINE_SESSION_ID` all
   left stdout, stderr and the exit status unchanged. The family needed
   `observed_iteration_cap()` — a `loop.max` document whose reported cap a
   leaked `CLAUDINE_MAX_ITERATIONS` replaces (3 → 1) — to be probeable at all.

5. **`common/pty.rs` builds no command.** The plan expected the session
   construction to live there; each of the three PTY binaries built its own, so
   each was migrated directly. `common/pty.rs` is untouched.

6. **One shape `build_std()` cannot express.**
   `sequence_overlay_pty`'s stdout-redirect test drives `/bin/sh -c` so the
   *shell* owns the `>`, making claudine a grandchild. The builder gained
   `apply_policy_to(&mut Command)` for it — the same computed `ChildEnvironment`,
   applied to a command the fixture did not build — and the claudine path comes
   from `command_std().get_program()` rather than a hand-rolled `cargo_bin`.

### AC3's named inherited-width cases, proven non-vacuous

`COLUMNS=44 cargo nextest run` on the two cases the predecessor's log recorded:

```console
$ COLUMNS=44 … -E 'test(inline_compose_wrong_type_prompt_takes_precedence_over_schema_scrub) + test(a_loop_accumulates_outputs_and_retains_mutations_across_iterations)'
     Summary [   0.603s] 2 tests run: 2 passed, 2506 skipped
```

And with the pre-migration file restored in place (`git show HEAD:… > …`, run,
restore, `diff` back to identical):

```console
$ COLUMNS=44 … -E 'test(a_loop_accumulates_outputs_and_retains_mutations_across_iterations)'
    ┃ loop limit exceeded for …/doc.md at iteration 3; cap is 3
     Summary [   0.868s] 1 test run: 0 passed, 1 failed, 2507 skipped
$ COLUMNS=44 … -E 'test(inline_compose_wrong_type_prompt_takes_precedence_over_schema_scrub)'
     Summary [   0.146s] 1 test run: 0 passed, 1 failed, 2507 skipped
```

So the pass is the migration's, not the host's.

### Neuter transcripts for the contamination probes

Each neuter was applied to the shared fixture, the probes run, the file restored
from a byte-for-byte backup, and the restore verified with `diff -q`.

| Neuter | Probes that fired |
|---|---|
| **A** — drop the inherited scrub loop in `child_environment` | `exported_render_width_and_color…`, `exported_claudine_application_variables…` |
| **B** — drop the `HOME`/`USERPROFILE` set | `exported_home_and_cache_relocation…` and every probe downstream of the fixture home (7 of 8) |
| **C** — return the parent's `PATH` verbatim from `PathPolicy::Minimal` | `an_exported_path_carrying_a_decoy_provider…` and 6 downstream |
| **D** — stop scrubbing `GIT_PLUMBING_VARS` in `helper_command` | `exported_git_plumbing_pointed_at_a_throwaway_repo…` |

An earlier version of C — appending the host `PATH` *behind* the fixture `bin` —
fired nothing, because the staged stub still won resolution. That is worth
recording: the probe discriminates "the child got the parent's `PATH`", not
"the child's `PATH` was long".

The `a_checkout_ancestor_temp_dir_is_refused_at_construction` probe is the one
that touches the checkout: it needs `TMPDIR` *inside* it to reproduce the
condition. It creates one directory under `target/` — gitignored build output —
and removes it on both the panicking and non-panicking paths.

### Guard arms whose behavior changed

Two, both because the burn-down reaching zero removed the live data they leaned
on.

- `deleting_a_spawn_entry_is_what_widens_the_isolation_population` asserted
  `spawn_allowlisted("sequence_cli.rs")`. `governs_isolation` is now
  parameterized (`governs_isolation_against`) and the test runs a synthetic
  before/after pair, so the rule stays provable with an empty live list — an
  assertion written against an empty list can only say "nothing is exempt",
  which a broken predicate says too.
- `an_empty_allowlist_leaves_every_live_site_unlisted` asserted the scan found
  *some* site. It is now
  `the_spawn_gate_reads_a_real_population_and_still_finds_a_planted_site`:
  population > 50 files including three named ones, a zero census, and a planted
  `cargo_bin("claudine")` that the same detector still finds and reconciles as
  unlisted.

### Validation checkpoint 5 — passed, with the Phase 4 host condition recurring

| Requirement | Status |
|---|---|
| `just test-cli` green | **yes** — 2506 passed / 10 skipped |
| `just test` green | **yes** — 6878 passed / 11 skipped |
| `just lint` green | **yes** |
| `just check-windows` green | **yes** — `x86_64-pc-windows-gnu --tests`, exit 0 |
| `just test-l2` green | **236 of 237** — the Atuin/WezTerm host condition, reproduced in isolation |
| burn-down artifact shows zero generic exemptions | **yes** — `{"kind":"total","gate":"spawn","files":0,"sites":0,"scanned_sites":0,"governed_files":90}` |
| isolation artifact | `…"scanned_sites":0,"governed_files":74"` — up from 37 |
| every contamination probe passes | **yes** — 8 of 8 |
| `git diff main -- .config/nextest.toml` empty | **yes** |

**Test count reconciliation, not netted.** Additions: 8 (the probe binary).
Removals: 0. Renames: 1 (the spawn gate's non-vacuity test — neither an addition
nor a removal). `just test-cli` 2498 → 2506 and `just test` 6870 → 6878 are
exactly the 8 additions.

### Pre-existing failures disclosed

- `completion_perf::perf_enter_compose_partial_meets_target` (`#[ignore]`d perf
  harness) fails on this host **before and after** the migration — verified by
  restoring the pre-migration file and re-running. Its PTY chooser reports
  `autocomplete requires an interactive terminal`. Not this phase's.
- `wrap_basics.rs` and `compose_caller_file_provenance.rs` carry unused-import
  warnings on the Windows target. Both are the predecessor's files and neither
  was touched here.

### Carried forward

- **Phase 6 owns the override census.** `context_reports_preserve_all_columns_at_minimum_supported_width`
  now runs in 1.74 s against a fixture repository rather than the monorepo; its
  30 s `slow-timeout` override is the first candidate to remove with the cost it
  was hiding.
- **`context_command.rs` is the launch-CWD attribution Phase 3 measured.** 24 of
  its 27 processes launched from the checkout; they now launch from an empty
  `git init`. Phase 8 should measure the family rather than assume the shape.
- **`OUT_OF_SCOPE` is retained but unused.** A new entry under it would be a
  regression, not a deferral; the constant's doc comment says so.

---

## Phase 6 — Non-spawn cost and test quality (RB3)

Ran after Phase 5's burn-down, on a 16-core Mac, 2026-09-08. Nothing here is
staged or committed: Phase 1's checkpoint is still blocked on the operator
merge, so the plan's rule that no Phase 4–10 *commit* may land first is intact.

### Headline

| Suite | Before | After |
|---|---:|---:|
| `just test-library` summed | 187.40 s | **102.00 s** |
| `just test-library` elapsed | 15.96 s | **8.89 s** |
| `error_guards` summed / processes | 20.20 s / 18 | **1.72 s / 8** |
| `composition::sequence::preflight` summed | 32.84 s | **2.21 s** |
| `composition::sequence::task` summed | 56.29 s | **20.43 s** |
| `linking::paths` summed | 9.31 s | **0.88 s** |
| `context_command` summed | 12.52 s | **11.55 s** |
| `.config/nextest.toml` override blocks | 24 | **16** |

### The finding that carried the phase

Phase 3 attributed the library's cost to `composition::sequence` and
`composition::schema` and left it there. Measured directly, the dominant term in
both is the same single call: `resolve_composition_source` opens with
`capture_file_resolution_context()`, which walks the **process CWD's**
repository topology before it looks at the reference it was given. Under
`cargo nextest` that directory is the monorepo checkout.

The decisive experiment ran one test binary by hand from two directories:

```text
composition::schema::tests::valid_required_property_passes
  cwd=claudine/lib   finished in 0.23s
  cwd=/tmp           finished in 0.01s

composition::sequence::preflight::tests::steps::mixed_steps_retain_identity_and_order
  cwd=claudine/lib   finished in 0.23s
  cwd=/tmp           finished in 0.00s

linking::paths::tests::new_populates_all_providers
  cwd=claudine/lib   finished in 0.59s
  cwd=<empty tmpdir> finished in 0.00s
```

Every fixture document is an absolute path inside its own `TempDir`, and
`ResolvedCompositionSource` carries no context forward — each downstream stage
re-anchors on the document's own parent. So the walk decided nothing, once per
test process, in three modules. One `#[cfg(test)]` seam,
`composition::resolve_fixture_source`, anchors the capture on the document's own
directory and panics on a relative path (a relative reference *would* answer
differently, and that difference must not pass silently).

`composition::sequence::preflight::tests::shell::bracket_target_identity_is_rejected_on_every_graph_shell_surface`
is the clearest single case: **6.083 s → 0.031 s**, because "every graph shell
surface" meant one ambient monorepo walk per surface.

### What was deliberately left on real discovery

- the whole `invocation_context` family — work-counter tests over repositories
  they build themselves;
- `cross_repo_task_nested_reference_uses_its_own_repository_context` and
  `resolve_repo_root_*`;
- `composition::schema`'s `make_source_in` / `serial(schema_validation_cwd)`
  tests, whose subject is precisely independence from the process CWD — routing
  them through the fixture seam would make them assert their own premise;
- `linking::paths::new_roots_the_table_at_the_process_home_and_the_resolved_repository`,
  added so `ProviderSkillPaths::new()`'s wiring keeps a test after the other
  eight stopped exercising it (0.64 s, and worth it: without it `new` could
  resolve both roots to `.` and every remaining test would still pass);
- both `shipped_implement_plan_*` tests, which keep the real
  `prompts/_implement/implement-plan.md` at its shipped path *and* its real
  `ctx.*` capture. The file declares no relative references, so relocating it
  would have bought 0.2 s and cost the library its only in-repository
  end-to-end preparation of a shipped prompt.

### `error_guards`: twelve processes, one scan

Attribution decision 2 asked for exactly one consolidation and named it. Twelve
of the eighteen cases called `scan_production_sources()`, whose `OnceLock` is
process-local, so under nextest each paid the full `syn` parse of `lib/src`,
`cli/src` and `contract/src`.

They are now named arms of `SCAN_GUARDS`, evaluated by one passive corpus test
that scans once and reports **every** failing arm under the test name it used to
carry. The six scan-free cases stay independently selectable — four of them are
the blindness anchors, and folding those in would make the corpus test its own
witness.

Non-vacuity, twice over:

1. `a_failing_scan_backed_guard_is_reported_under_its_own_name` runs the guard
   table against a synthetic scan holding one planted collapse and no trait
   impls, and asserts both which arms must name themselves and which must stay
   quiet — an aggregator that swallowed a failure, or reported it without
   saying which contract broke, would make all twelve pass by being silent.
2. A live neuter. Appending a stale entry to `transport-allow.toml`:

   ```text
   1 of 12 source-backed diagnostic guard(s) failed:

   ── every_allowlist_entry_still_matches_a_live_site
   1 allowlist entr(y/ies) match no live site — delete them:
     [formatted_report] lib/src/nonexistent_neuter.rs in `neuter_probe`
   ```

   File restored and diffed back identical.

### Override census

Eight blocks removed, none added. `git diff main -- .config/nextest.toml` is
16 insertions / 41 deletions and **every inserted line is a comment**:

```text
$ git diff main -- .config/nextest.toml | grep '^+' | grep -v '^+++' \
    | grep -v '^+#' | grep -v '^+$'
(no output)
```

The verdict table with before/after numbers is in
[`inventory.md` § Phase 6 disposition](inventory.md#phase-6-disposition). The
new finding is that **every per-test `slow-timeout` override in the `ci` profile
was a no-op** — all eight set `{ period = "30s", terminate-after = 3 }`, which
is that profile's own default. The profile comment now records the rule.

### Assertion and identity repairs

Seven, each with its before/after recorded at the call site; the list is in
`plan.md`. Two carried neuter transcripts:

```text
# wrap_opencode: needle changed to `Quota limit reached`
error field must carry the provider's rate-limit message, not a generic
failure; got "Usage limit reached for glm-5.1 (zai-coding-plan); resets at
2026-04-15 21:18:56"

# event_renderer: silent gate hoisted above the api_key_source self-update
session_start_updates_the_auth_source_even_when_silent
assertion `left == right` failed
  left: None
 right: Some("ANTHROPIC_API_KEY")
```

Both files diffed back identical afterwards. The second is worth naming: the
ordering it pins is called out in `render`'s doc comment and had no test at all,
so replacing a tautology (`assert_eq!(Verbosity::Silent, Verbosity::Silent)`)
produced net new coverage rather than a smaller suite.

### Recipe and metadata reconciliation

`affected_scope.py` validates the five new `[package.metadata.ci.tests]` blocks,
and still rejects a bad one:

```text
$ printf 'tiers-typo = ["L1"]\n' >> claudine/rendezvous/core/Cargo.toml
$ python3 scripts/ci/affected_scope.py --all
RuntimeError: package 'rendezvous-core' [package.metadata.ci].tests has unknown
field(s): ['tiers-typo']; allowed: ['all-features', 'companion-suites',
'features', 'l1-include-slow', 'l2-backends', 'local-features', 'runner-tools',
'tiers']
```

File restored and diffed back identical; the validator is green again.

The two `rendezvous-daemon` mDNS identities that ran in no recipe at all now
run in one:

```text
$ cd claudine/rendezvous && just test-real
    PASS [   0.204s] (1/2) rendezvous-daemon::peer_discovery real_two_daemons_discover_each_other_via_mdns
    PASS [   0.204s] (2/2) rendezvous-daemon::peer_discovery real_mdns_discovered_peer_cannot_sync_before_approval
     Summary [   0.204s] 2 tests run: 2 passed, 169 skipped
```

### Disclosed failures

- **`just test-real` from the `claudine` area: 4 of 5 fail on this host.** The
  four `claudine-contract::real_provider` identities report `Unauthorized` —
  the local provider CLI is not authenticated here. They fail identically under
  the retired `cargo test` form, so this is a pre-existing host condition, not
  a regression from moving the recipe onto nextest. Worth a separate look: the
  file's contract says each test "skips cleanly when its provider/model is
  unavailable", and an expired credential is evidently not on that path.

### Checkpoint 6

| Gate | Result |
|---|---|
| `just test` | 6871 passed / 9 skipped, 24.86 s |
| `just test-cli` | green |
| `just test-gen` | green (fleet: 83 records, 0 failures) |
| `just test-contract` | green |
| `just doctest` | 25 passed / 7 ignored |
| `just bench` | exit 0 (`BENCH_YES=1`, see below) |
| `just lint` | exit 0, claudine area and `claudine/rendezvous` |
| `just test-rendezvous` | green — core, daemon, client |
| `just check-tier-coverage` | exit 0, **0 stranded** |
| `just check-test-interrupts` | exit 0, 34 areas |
| `just check-windows` | exit 0 |

`just bench` refused three times on `_bench_preflight`'s 1-minute load-average
check (11–33 against a threshold of 8) while `/usr/bin/top` showed the CPU
idle — the recorded AutoMounter/SMB load-average inflation on this host. With
`BENCH_YES=1` the run completed: exit 0, zero errors, zero panics.

### `just test-l2`: 236/237, same survivor as Phases 4 and 5

Four runs, and the fail-fast default made the first three misleading:

| Run | Mode | Result |
|---|---|---|
| 1 | default (fail-fast) | 174/237 run — `level2_shipped_implement_plan_supplied_commit_message_runs_exact_commit_branch` |
| 2 | default | 174/237 run — same test |
| 3 | default | 162/237 run — `LEAK-FAIL` on `level2_wezterm_file_property_uses_choose_one` (exited 0, leaked handles) |
| 4 | `--no-fail-fast` | **237 run, 236 passed** — `level2_initialize_proxy_block_auto_detects_osc8_in_wezterm` |
| 5 | `--no-fail-fast` | **237 run, 236 passed** — same |

The run that actually executes the tier reproduces exactly the failure Phases 4
and 5 recorded (Atuin's first-run prompt sitting in the spawned WezTerm pane and
swallowing the exit marker), in a `level2_*` file this phase never touched.
`level2_shipped_implement_plan_supplied_commit_message_runs_exact_commit_branch`
passes in isolation and does not reproduce in either full run.

Nothing this phase changed can reach an L2 binary, which is the structural half
of the argument: the library edits are all `#[cfg(test)]` items or test modules,
and those are excluded when `claudine` is compiled as `claudine-cli`'s
dependency; no `cli/tests/common/` file and no `level2_*` file was touched; and
of the eight `.config/nextest.toml` blocks removed, none matched a `level2_`
filter — the `package(claudine-cli) & test(/level2_/)` blanket is intact.

### Test count reconciliation, not netted

6878 → 6871 is exactly −7:

| Change | Δ |
|---|---:|
| `error_guards` 18 → 8 (twelve scan-backed identities merged into one, plus one new non-vacuity test) | −10 |
| `linking::paths` 10 → 11 (`new_roots_the_table_…` added) | +1 |
| `render::event_renderer` (two replacements for the deleted tautology) | +2 |
| `stream::stderr` (the tautology) | −1 |
| `context_command` 27 → 26 (the duplicate `--values` test) | −1 |
| `wrap_sigint` — `slow_` rename enters L1 selection | +1 |
| `claudine-gen::signals_validation` — `real_` rename enters L1 selection | +1 |

Skips 11 → 9 is the same two renames leaving the filtered-out set.

### Carried forward

- **Phase 7 owns what is left at the top of both suites.** After this phase the
  library's most expensive identities are `composition::sequence::task`'s shell
  process trees (nine at 1.6–2.1 s) and `config::atomic::concurrent_writers`;
  the CLI's are `sequence_overlay_pty` (18.62 s / 7) and the two
  `wrap_watchdog_*` binaries (21.95 s / 11). All of them are sleeps, deadlines,
  or real child trees — Phase 7's subject, not this phase's.
- **`compose_caller_file_provenance` (9.35 s / 15) is next after those.**
  Fifteen distinct proxy/caller-identity scenarios at the boundary that proves
  them; no obvious shared setup to hoist. Phase 8 should measure before anyone
  assumes there is.
- **The `context` reports have no in-process render seam.** `render_default_report`
  and friends write to `log::data`, so a width sweep cannot move below the CLI.
  Adding a capture seam is a production change and stays out of scope; if a
  later fix wants the width matrix cheaper, that seam is the prerequisite.

---

## Phase 8 — Local measurement (RB5, first evidence tranche)

**Outcome: complete. Every number in this section is local and is attribution
only; it establishes no CI target.** Phases 4–7 are implemented and committed on
`fix/cli-slow-tests` except Phase 7's library test edit, which is in the working
tree; Phase 1's CI checkpoint is still blocked on the operator merge, so the CI
tranche (Phase 9) remains pending.

### Validation record — recorded once, reused by Phases 9–10

Reuse this evidence while `claudine/**` source, `.config/nextest.toml`,
`just/devops.just`, `claudine/justfile` and the toolchain below are unchanged;
a change to any of them re-runs only the affected suite.

| Field | Value |
|---|---|
| Commands | `just test` (five crates, one nextest invocation); `just test-rendezvous` (three invocations); `BISCUIT_L2_THREADS=8 just _test_l2 claudine-cli --features terminal-tests --test level2_dry_run_pty --test level2_provided_partial_file_pty --test level2_pty_tests --test level2_schema_prompt_pty` — the `test-l2` recipe's own parallel self-spawn path, narrowed to the four PTY binaries because `test-l2` would also run `claudine-gen`'s L2 set, where none of this phase's targets live |
| Selection | `_tier_filter L1` for the first two; `_tier_filter L2` for the third. `BISCUIT_TEST_FILTER` unset |
| Features | none (L1); `terminal-tests` (L2 PTY) |
| Profile | `default` — `NEXTEST_PROFILE` and `BISCUIT_CI_ENVIRONMENT` unset, so no CI test-group cap applies and nextest schedules 16-way |
| Platform | aarch64-apple-darwin, macOS 27.0 (26A5425a), Apple M4 Max, 16 cores, 128 GiB |
| Toolchain | rustc 1.97.1 (8bab26f4f 2026-07-14), cargo-nextest 0.9.136, just 1.56.0, node v22.20.0, `RUSTC_WRAPPER=kache` |
| Baseline source | detached worktree `/tmp/rb-baseline-9fc5151a0` at `9fc5151a0` (the revision Phase 1's local gates ran at), clean, own `target/` |
| Candidate source | this worktree, `fix/cli-slow-tests` @ `dabbeca02`, dirty: `claudine/lib/src/composition/sequence/task/tests.rs` (Phase 7, uncommitted) plus `.claude/skills/claudine/SKILL.md`, `.claude/skills/rust-testing/SKILL.md` and this fix's documents |
| Environment | ambient `CLAUDINE_INTERACTIVE`, `CLAUDINE_PID`, `CLAUDINE_SESSION_ID` from the agent session (the builder scrubs `CLAUDINE_*` from every child; Phase 5's `exported_claudine_application_variables_do_not_change_the_result` covers the namespace); no `NEXTEST_*`, `BISCUIT_*`, `MODEL`, `PLAYA_*`, `NO_COLOR`, `FORCE_COLOR`. `GIT_TERMINAL_PROMPT=0` exported by the runner |
| Cache state | both build directories warm before any counted run. The uncounted warm-ups absorbed 175 (`just test`, baseline), 28 (candidate), 298 (`just test-rendezvous`, baseline) and 1 (L2 PTY) crates: each recipe unifies features for its own package selection, so a raw eight-package `nextest run --no-run` had not produced the five-package artifacts. Every counted run reports `Compiling` ×0 and `Finished … in 1.1–2.9 s` |
| Concurrency and load | one recipe at a time, never two. No cargo, nextest or editor work of mine ran alongside; a `claudine codex --yolo` session from another worktree was present and idle (0 % CPU). System daemons were not idle: `identityservicesd` 30–90 %, `WindowServer` ~50 %, `coreaudiod` ~26 %, `Snagit` ~22 % of one core each at various points, i.e. two to four cores of noise the runs could not exclude. Load average and the second `top` sample's CPU idle are recorded before every run in `runs.jsonl` |
| Result | 55 recipe runs, **all exit 0**, 0 failed / timed out / leaked / retried tests |
| Artifacts | [`measurement/provenance.json`](measurement/provenance.json), [`measurement/runs.jsonl`](measurement/runs.jsonl), `measurement/runs/*.log.gz` (verbatim recipe output, gzipped), [`measurement/report.md`](measurement/report.md); series 2 under [`measurement/series-2/`](measurement/series-2/); sentinels under [`measurement/sentinels/`](measurement/sentinels/) |

Tooling, all in this directory and all `node:test`-covered like the earlier
gates: [`measurement-runner.ts`](measurement-runner.ts) drives the plan
([`measurement/plan.json`](measurement/plan.json)) and refuses to start on a
cold build directory; [`measurement.ts`](measurement.ts) reads the logs, keeps
the three costs apart, and **exits 1** on a truncated log, a count that
disagrees with nextest's own summary, an identity set that is not stable across
one revision's runs, a target with fewer than ten executions, or any
non-passing result; [`sentinels.ts`](sentinels.ts) takes the work counters.

```bash
npx tsx --test measurement.test.ts                      # 10 tests
npx tsx measurement-runner.ts --plan measurement/plan.json --out measurement
npx tsx measurement.ts report --manifest measurement/runs.jsonl \
    --cohorts measurement/cohorts.json --targets measurement/targets.json
npx tsx sentinels.ts --baseline /tmp/rb-baseline-9fc5151a0 --candidate "$(git rev-parse --show-toplevel)" \
    --out measurement/sentinels                         # re-reads saved transcripts
```

### Protocol, and why it ran twice

Per suite and revision: one uncounted warm-up, then `baseline, candidate,
baseline, candidate, …` five times, then five candidate-only load rounds and
ten L2 PTY rounds (series 1, 45 runs, 13:15–13:41 local). The host was not
quiet during the `just test` alternation: the first counted baseline run took
88.6 s of runner elapsed against a 49–55 s neighbourhood, and the CPU-idle
reading before its candidate neighbour was 0.97 %. The whole-series drift
bracket that produces — 74 % for the baseline, 57 % for the candidate — is wider
than the effect, so the strict rule (a median delta must clear both revisions'
max − min) reports the whole-suite delta as *not established* even though every
pair improved. A second alternating series of ten `just test` runs was taken
50 minutes later in a quieter window (13:43–13:51, CPU idle 64–83 % before
every run); its bracket is narrower (42 % / 32 %) but the host still drifted
upward mid-series (baseline runs 37 → 59 s), and the strict rule still fails.

Both series are reported. The reading that survives both is the **paired**
one, which is what alternation exists to produce: each baseline run has a
candidate neighbour taken under the same host state, and the candidate ÷
baseline ratio per pair is insensitive to drift the whole-series bracket cannot
separate from the effect.

### Headline — `just test`, three costs apart

| | Baseline (9fc5151a0) | Candidate | Paired ratio min / median / max |
|---|---:|---:|---|
| Build/setup (wall − elapsed), series 2 | 1.5–1.9 s | 1.6–1.7 s | — |
| Runner elapsed, series 2 medians | 52.30 s | **37.89 s** | 0.643 / **0.712** / 0.765 |
| Summed test duration, series 2 medians | 819.42 s | **587.89 s** | 0.637 / **0.707** / 0.758 |
| Runner elapsed, series 1 medians | 53.44 s | 36.01 s | 0.563 / 0.653 / 0.876 |
| Summed test duration, series 1 medians | 836.68 s | 558.03 s | 0.566 / 0.646 / 0.863 |
| Identities run | 6861 | 6873 | +28 / −16 |
| Skipped | 11 | 9 | the two renamed tier identities now run |
| Failed / timed out / leaked / retried | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 | every run |
| Slow marks (> 5 s) per run | 1–58 | 0–10 | series 1; 1–20 vs 0–1 in series 2 |

Improved in **all ten pairs** across both series. The magnitude carries the
host's noise — roughly −25 % to −36 % elapsed depending on the pair — and the
number to quote is the paired median, **0.71**, not the whole-series delta.
Summed duration moved by the same ratio as elapsed, which is the column that
matters for the CI leg that runs `claudine-cli` at `max-threads = 1`
(attribution.md § The concurrency fact).

`just test-rendezvous`: paired median 0.96 elapsed / 0.95 summed, min–max
0.66–1.08 — **no change, and none was claimed**; Phase 7 added one test there
(272 → 273) and touched no timing.

### Changed cohorts, from the same suite reports

Series 2, medians of summed duration across the five alternating runs of each
revision; *established* means the median delta clears both revisions' own
max − min. Full table with min/max in [`series-2/report.md`](measurement/series-2/report.md);
series 1's in [`report.md`](measurement/report.md) agrees in direction on every
row.

| Cohort | Tests B → C | Summed B → C | Delta | Established |
|---|---|---:|---:|---|
| lib `composition::sequence::preflight` | 44 → 44 | 45.30 → 4.05 s | −91 % | yes |
| lib `composition::sequence::task` | 106 → 106 | 70.38 → 15.35 s | −78 % | yes |
| lib `composition::schema` | 75 → 75 | 58.95 → 14.43 s | −76 % | yes |
| lib `linking::paths` | 10 → 11 | 14.74 → 2.09 s | −86 % | yes |
| `error_guards` | 18 → 8 | 60.13 → 3.25 s | −95 % | yes |
| `context_command` | 27 → 26 | 43.79 → 12.79 s | −71 % | yes |
| `sequence_overlay_pty` | 7 → 7 | 18.11 → 4.23 s | −77 % | yes |
| Phase 5C context / errors / completion family | 84 → 83 | 52.66 → 21.36 s | −59 % | yes |
| Phase 5D live-child and PTY cohort | 11 → 12 | 20.46 → 6.23 s | −70 % | yes |
| guards, probes, fixture self-tests | 62 → 71 | 69.82 → 21.90 s | −69 % | inside drift (series 1: −75 %) |
| Phase 5A compose family | 69 → 69 | 20.54 → 16.34 s | −20 % | inside drift |
| Phase 5B sequence / loop family | 155 → 155 | 49.09 → 46.66 s | −5 % | inside drift |
| all `claudine-cli::*` integration binaries | 2489 → 2498 | 417.36 → 310.66 s | −26 % | inside drift |
| all `claudine` lib tests | 4040 → 4042 | 361.31 → 220.35 s | −39 % | inside drift |
| `claudine-cli` bin unit tests (untouched) | 1694 → 1694 | 81.83 → 89.69 s | +10 % | inside drift — the control |
| contract / catalog-types / gen (untouched) | 223 → 224 | 27.91 → 29.35 s | +5 % | inside drift — the control |

The two untouched cohorts are the controls: they move by +5–10 % between
revisions under the same host, which is the size of the noise, and the
established rows move by −59 % to −95 %. The Phase 5A/5B families were migrated
to the fixture for isolation (RB2), not for speed; the plan attributed no
timing claim to them and none is made here. **No cohort was run in isolation.**

The identity change is +28 / −16, every one named in the report's *Identity
changes* list, and reconciles to Phases 4–7's own records: +8 `cli_process_fixture`
self-tests, +8 `contamination_probes`, +4 `spawn_site_guard`, +2 `error_guards`
(the corpus test and its non-vacuity witness) against −12 merged
scan-backed identities, +2 `event_renderer` against −1 `stream::stderr`
tautology, `linking::paths` +2 / −1 (one rename, one addition), −1
`context_command` duplicate, and the two renamed tier identities
(`compose_sigint_during_prep_exits_130_with_notice`,
`shipped_corpus_builds_deterministically`) entering L1. The rendezvous
endpoint test is the +1 there.

### Ten executions of every changed timing / concurrency contract

Target set: [`measurement/targets.json`](measurement/targets.json) — 13
entries, 36 identities. Each ran **eleven** times on the candidate: one
warm-up, five alternating, five load rounds. The representative load is the
full L1 population of the target's own package set (`just test` for the
library and CLI targets, `just test-rendezvous` for the endpoint test); for
the four L2 PTY binaries it is the four running together at `-j 8`, which is
exactly the concurrency the removed `serial(pty)` group used to forbid.
Migrated-but-unchanged files (`wrap_sigint`, `handle_deadline`,
`compose_ttff_perf`, `completion_perf`) were not re-run separately.

| Target | Identities | Executions | Min | Median | Max | Non-passing / retries |
|---|---:|---:|---:|---:|---:|---|
| `composition::sequence::task` reap waits → pid observation (4) | 4 | 44 | 0.038 s | 0.049–0.067 s | 0.106 s | 0 / 0 |
| `…::the_system_shell_interrupts_a_running_tree` marker wait | 1 | 11 | 0.062 s | 0.075 s | 0.088 s | 0 / 0 |
| `sequence_overlay_pty` (OSC 10/11 answered; concurrent) | 7 | 77 | 0.266 s | 0.34–0.75 s | 1.211 s | 0 / 0 |
| `level1_compose_autocomplete_failure_pty`, `level1_inline_compose_mismatch_pty` | 4 | 44 | 0.243 s | 0.32–0.59 s | 0.848 s | 0 / 0 |
| `rendezvous-daemon::pairing_and_sync::endpoints_are_stable_per_fixture_and_distinct_across_fixtures` | 1 | 11 | 0.287 s | 0.354 s | 1.062 s | 0 / 0 |
| four L2 PTY binaries at `-j 8` | 19 | 209 | 0.279 s | 0.29–4.63 s | 4.713 s | 0 / 0 |

396 executions, 0 non-passing, 0 retries, 0 leaks. One outlier is recorded
rather than smoothed: `level2_pty_provided_partial_single_match_confirms_and_launches`
took 4.609 s once against a 0.81 s median (its neighbour
`level2_pty_dry_run_approval_prompt_matches_normal_mode` sits at 4.6 s every
time, so a scheduling collision at `-j 8` is the likely cause), and passed.
Per-identity rows are in [`report.md`](measurement/report.md).

### Cold-build claims

None exist. Phases 4–7 made no cold-build claim (`log.md` and the plan carry
no such statement outside the Phase 8 bullet itself), so there was nothing to
measure. The baseline worktree's isolated build directory was created for the
alternation, not for a claim; the developer's working cache was never cleared.
For the record only: the raw eight-package `nextest run --no-run` there took
3 m 18 s through kache, and the recipes' own feature-unified builds a further
1 m 37 s (`just test`) and 56 s + 1 m 14 s + 3 s (`just test-rendezvous`).

### Work counters and sentinel effects, independent of timing

[`sentinels.ts`](sentinels.ts); raw transcripts and the summary in
[`measurement/sentinels/`](measurement/sentinels/). The counters are lldb
breakpoint hit counts at the **entry location** of the function each claim
says is no longer called, on the same test binaries the suite ran (resolved
with the suite's own five-package selection so feature unification names the
same artifact — the script fails if `cargo` compiles anything). A regex
breakpoint also lands on the closures a function instantiates, which fire once
per call each; only the entry location in the defining file is counted, and
every other location is listed in the record. libtest colours its summary
under lldb's terminal, which the first pass of the parser missed; the
transcripts were re-read rather than re-run.

**S1 — the ambient CWD walk in the redirected library modules** (Phase 6's
`resolve_fixture_source`). One process per module, `--test-threads=1`, every
test passing at both revisions:

| Module (tests) | `capture_file_resolution_context` B → C | `GitRepo::discover` B → C | `resolve_repo_root` B → C |
|---|---:|---:|---:|
| `composition::schema` (75) | 73 → **11** | 111 → 49 | 0 → 0 |
| `composition::sequence::preflight` (44) | 62 → **2** | 65 → 5 | 0 → 0 |
| `composition::sequence::task` (106) | 84 → **0** | 84 → 0 | 0 → 0 |
| `linking::paths` (10 → 11) | 0 → 0 | 10 → 3 | 10 → **3** |

The survivors are the calls Phase 6 said it kept: `schema`'s
`make_source_in` / `serial(schema_validation_cwd)` tests and the two
`shipped_implement_plan_*` cases (11), `preflight`'s
`cross_repo_task_nested_reference_uses_its_own_repository_context` (2), and in
`linking::paths` the two `resolve_repo_root_*` tests plus
`new_roots_the_table_at_the_process_home_and_the_resolved_repository` (3).
`discover` in `schema` stays at 49 because the retained tests build and
discover their own repositories — that is their subject.

**S2 — one production scan per `error_guards` process.** `run_scan` is the
`OnceLock` initializer behind `scan_production_sources`; counted per identity
in its own process, as nextest runs it, and summed:

| | Identities | Processes that ran `run_scan` |
|---|---:|---:|
| baseline | 18 | **12** |
| candidate | 8 | **1** |

**S3 — where `context_command` launches from.** A `git` shim first on `PATH`
logged every parent-side invocation with its working directory while
`just test-cli --test context_command` ran at each revision (27 → 26 tests,
all passing):

| | `git init` | of which inside the checkout | `git rev-parse --show-toplevel` from the checkout root | `current_dir(repo_root())` sites | `repository_fixture()` sites |
|---|---:|---:|---:|---:|---:|
| baseline | 0 | 0 | **44** | 23 | 0 |
| candidate | **34** | **0** | 0 | 0 | 23 |

The baseline located the monorepo 44 times and launched every context command
from it; the candidate built 34 repositories under `$TMPDIR` and never asked
where the checkout was (the two remaining `rev-parse` calls at each revision
are the recipe's own). Phase 6's hoisting shows in the count too: 26 tests,
34 inits — the two width sweeps build one repository each instead of 12 and 7.

**S4 — no audio and no survivors.** `just test-leaks claudine` from the repo
root: **7146 passed / 11 skipped, `leak-sweep: no leaked processes detected`** —
the same sweep that found two orphaned `claudine` audio workers before Phase
7's `PLAYA_DRY_RUN` default. The baseline sweep was **not** re-run: Phase 7 recorded that
it leaves two `claudine` processes behind and plays a sound through the host's
speakers, and repeating a known side effect on the developer's machine buys
no evidence the record does not already hold.

**S5 — the structural counters that run in every suite run.**
`just test-cli --test spawn_site_guard --no-capture` on the candidate: 18
passed, and the gates wrote
`{"kind":"total","gate":"spawn","files":0,"sites":0,"scanned_sites":0,"governed_files":90}`
and
`{"kind":"total","gate":"isolation","files":0,"sites":0,"scanned_sites":0,"governed_files":74}`
(copied into `measurement/sentinels/`), against the baseline's recorded 36
files / 172 raw spawn sites across 89 governed files and 0 escapes across 37.
Both gates execute inside every one of the 16 candidate `just test` runs above,
so "zero raw spawn sites" and "zero isolation escapes" were re-proved 16 times
during measurement, not once.

### Gates

Tests before lint, per Phase 1's stale-binary note. The candidate `just test`
and `just test-rendezvous` gates are the measurement runs themselves.

| Gate | Area | Result |
|---|---|---|
| `just test` ×16 (candidate: series 1 warm-up, five alternating, five load; series 2 five alternating) | `claudine` | 6873 passed / 9 skipped, exit 0, every run |
| `just test-rendezvous` ×11 (candidate) | `claudine` | 273 passed / 2 skipped, exit 0, every run |
| `just _test_l2 … --test <4 PTY binaries>` ×11 (candidate) | `claudine` | 19 passed, exit 0, every run |
| `just test-leaks claudine` | repo root | 7146 passed / 11 skipped, `leak-sweep: no leaked processes detected` |
| `npx tsx --test measurement.test.ts` | fix directory | 10 passed, 0 failed |
| `measurement.ts report` (series 1, series 2) | fix directory | `GATE EXIT=0`, `GATE EXIT=0` |
| `sentinels.ts` | fix directory | `SENTINELS EXIT=0` |
| `just test` | `sniff` | 2599 passed / 23 skipped, exit 0, 48.2 s |
| `just lint` | `sniff` | exit 0 |
| `git diff main -- .config/nextest.toml` | repo root | 16 insertions / 41 deletions, 0 non-comment insertions (AC5, AC7) |

`sniff` is included because the session was started in that area; no `sniff`
file was read or written by this phase. No Rust source changed in this phase,
so `just lint` in `claudine` is Phase 7's record.

### Deliberate choices

- **Two series, both reported.** Dropping series 1 would have selected the
  quieter evidence; it stays, with its bracket, and the paired reading is what
  reconciles them.
- **Strict bracket and paired ratio side by side.** The strict rule is kept
  because it is the one that cannot be gamed by drift in the effect's favour;
  the paired ratio is added because the protocol's alternation is *for* it.
  Where they disagree, the log says so rather than picking.
- **Warm-ups counted toward the ten executions.** They are compatible runs —
  same binary, same load cohort, the compile happens before the first test
  starts — so eleven executions are reported rather than ten.
- **Logs gzipped.** 4.5 MB for 55 verbatim runs instead of ~50 MB; the parser
  reads either form and the raw per-test vectors are intact.
- **The load cohort is the whole suite.** A hand-picked "representative"
  subset would have been a claim about representativeness; the population the
  targets actually ship in is not.
- **lldb entry-location counts, not aggregate hit counts**, after the first
  pass showed a regex breakpoint's aggregate inflated 6.5× by closure
  locations (474 against 73 real calls). The raw transcripts show both.

### Carried forward

- Phase 9: the CI tranche is the only source for budgets; nothing here may
  feed `deriveBudgets`. The paired-ratio reading (0.71 elapsed and summed) is
  the local prior for what the `claudine-cli` `max-threads = 1` leg should
  show in its *summed* column, and only that.
- Phase 10: `results.md` can be assembled from `measurement/report.md`,
  `series-2/report.md` and `sentinels/summary.tsv` without re-running
  anything, provided the reuse rule at the top of this section still holds.
- Phase 10 skill candidates (`rust-testing`), recorded here rather than
  edited in: (a) lldb entry-location hit counts as a work counter for
  "this call was removed" claims — no root, no instrumentation, and the
  transcript is the evidence; (b) the per-invocation feature-unification
  warm-up: a raw `nextest run --no-run` over a wider package set does **not**
  warm the artifacts a narrower recipe invocation will build; (c) narrowing
  `test-l2` to named binaries needs `just _test_l2 <pkg> --features … --test …`
  because the public recipe fans out to a second package.
- The host: `identityservicesd` at 30–90 % of a core for most of an hour is
  the same class of background noise the 2026-08-13 triage recorded; it is
  disclosed, not diagnosed, here.

---

## Phase 9 — CI evidence (RB5, second tranche; AC6)

**Outcome: partial, and human-gated exactly where the plan said it would be.**
The operator action Phase 1 was blocked on has happened — the predecessor
merged to `main` — so the baseline half of the CI tranche is open and its
first run is collected, gated and stored. The candidate half cannot start: the
branch has 36 unpushed signed commits plus this working tree, a merge of
`origin/main` conflicts in thirteen files, and committing, merging and pushing
are operator actions this non-interactive session cannot take. Everything that
does not depend on a push is done: the consolidated local validation, the
Windows check, the gate over the stored baseline, the gate's two hardening
changes this run forced, the handoff, and the budget answer. Nothing below
reports a pending thing as passing.

### Grounding facts re-checked (2026-09-08, 14:00–15:30 UTC)

- **PR #69 merged.** `fix/cli-slow-tests` at `a9e88c069` (the predecessor's
  nine commits) merged to `main` as `444213eb5` at 00:27 UTC. `git diff
  a9e88c069 444213eb5` is empty: the merge tree *is* the PR-head tree.
- **`main` moved again thirteen hours later.** PR #70 (`feat/unifi`) merged as
  `6504747e2` at 13:29 UTC, changing `claudine/lib` (composition closure /
  completion / schema), four `claudine/cli/tests` files
  (`wrap_inline_compose.rs` +595, `wrap_perf.rs`, `shipped_prompt_contract.rs`,
  `wrap_inline_compose_interactive.rs`), `claudine/justfile`'s `test-real`
  recipe, and `scripts/ci/test_affected_scope.py`. A `main` push after that is
  a different source state from the merge commit.
- **Runs on `main`:** `34173378609` (`ci`, push, `444213eb5`) completed green at
  02:44 UTC; `34232285291` (`ci`, push, `6504747e2`) was queued at 13:29 UTC; by 15:05 UTC
  its three native `claudine-cli` L1 legs had completed green and the WSL2 and
  L2 legs were still queued — recorded, not collected, since it is a
  different source state (below). The PR's own
  `pull_request` run `34159725015` at `a9e88c069` (20:31 UTC the day before)
  is green on the identical tree.
- **This branch:** local HEAD `973d1d918`, 36 commits ahead of
  `origin/fix/cli-slow-tests` (which still sits at the PR head `a9e88c069`),
  every one OpenPGP-signed by `Ken Snyder <ken@ken.net>`. No Rust file under
  `claudine/**`, `.config/` or `just/` differs from HEAD in the working tree,
  so Phase 8's validation record is reusable under its own rule; only this
  phase's documents and the gate script are new.
- **The Windows toolchain is present** (`x86_64-pc-windows-gnu` target,
  `/opt/homebrew/bin/x86_64-w64-mingw32-gcc`, mingw-w64 14.0.0).

### The baseline's first run, gated

Collected with the recipe `baseline/README.md` fixed in Phase 1; the gate's
verbatim output sits beside each run as `junit-metrics.txt`.

| Run | Event | Source | Legs | Gate |
|---|---|---|---|---|
| `baseline/34173378609/` | push to `main` | `444213eb5` | four, all green | **exit 0** — baseline run 1 of 3 |
| `baseline/34159725015/` | `pull_request`, PR #69 | `a9e88c069`, tree-identical | four, all green | **exit 0** — supplementary, not counted |

The three costs, from the `main` run:

| Environment | Build/setup | Runner elapsed | Summed duration | Tests | Failures | Skips |
|---|---:|---:|---:|---:|---:|---:|
| `ubuntu-latest` | 695.1 s | 324.9 s | 324.7 s | 2466 | 0 | 0 |
| `macos-latest` | 832.3 s | 753.7 s | 753.5 s | 2466 | 0 | 0 |
| `windows-latest` | 1012.9 s | 356.1 s | 355.8 s | 2105 | 0 | 0 |
| `wsl2-ubuntu` | 81.3 s | 692.7 s | 692.2 s | 2466 | 0 | 0 |

and from the tree-identical PR run, which is the only run-to-run noise bracket
the baseline has until its second `main` sample exists:

| Environment | Build/setup | Runner elapsed | Summed duration | Matched summed, PR ÷ main |
|---|---:|---:|---:|---:|
| `ubuntu-latest` | 222.8 s | 301.2 s | 301.0 s | 0.927 |
| `macos-latest` | 273.4 s | 794.6 s | 794.3 s | 1.054 |
| `windows-latest` | 878.2 s | 304.8 s | 304.3 s | 0.855 |
| `wsl2-ubuntu` | 65.9 s | 690.1 s | 689.6 s | 0.996 |

Three readings, all of which Phase 3 predicted:

- **Runner elapsed equals summed duration** on every leg to within a second,
  because `claudine-cli`'s CI profile runs at `max-threads = 1`. On CI the
  summed column is the floor, not something parallelism hides.
- **The build column is the largest cost on three legs** (695–1013 s against
  325–754 s of tests) and near zero on WSL2, whose `nextest archive` is built by
  the host job. The recipes still cannot separate build from run locally;
  `manifest.duration_s − <testsuites time>` is what makes the column exist.
- **The same tree varies 7–15 % run to run** on Ubuntu and Windows and 5 % on
  macOS. A candidate ratio inside that bracket on one run is not a result.

**Identities.** 2466 on each Unix leg, the same set on all three. 2105 on
`windows-latest`: 372 Unix-only identities (every `#![cfg(unix)]` binary) and
11 Windows-only ones, including the two live-child console-control tests no
local host could run — `wrap_ctrl_c_windows::ctrl_c_terminates_wrapped_child_on_windows`
(1.25 s) and `sequence_ctrl_c_windows::sequence_ctrl_c_fans_out_to_parallel_children_on_windows`
(4.02 s), both passing. That closes the predecessor's finding 3 outright
(recorded in its `deferred-performance.md`, 2026-09-08 addendum).

**Timeout-shaped tests.** All nine inside budget + tick + allowance on the three
Unix legs; the thinnest margin is `sequence_per_step_step_timeout_override` at
1.3 s against its 1.6 s bound on macOS (1.2 s on Ubuntu; 1.2 s against 2.6 s on
WSL2). Floors are printed, not enforced, for the baseline, as Phase 1 set.

**Slow cases, tracked so that lost coverage cannot read as speed** (`main` run):

| Environment | ≥ 2 s | ≥ 5 s | Slowest |
|---|---:|---:|---|
| `ubuntu-latest` | 34 | 15 | `context_reports_preserve_all_columns_at_minimum_supported_width` 19.06 s |
| `macos-latest` | 30 | 1 | same, 9.71 s |
| `windows-latest` | 42 | 5 | `shipped_implement_router_keeps_the_callers_launch_origin_for_its_lazy_target` 16.17 s |
| `wsl2-ubuntu` | 34 | 19 | `context_reports_preserve_all_columns_at_minimum_supported_width` 62.44 s |

The `context_*` and `compose_eager_spec_setter_*` cases at the top of every leg
are the launch-origin discovery cohort Phase 6 removed from the library and
Phase 5 removed from the CLI fixtures; the candidate's per-environment
comparison is where that claim gets its CI number.

### What the first run taught the gate

Running `junit-metrics.ts` over the real artifacts, with Phase 1's
`expectations.json`, **failed** — eleven `missing-test` violations, all on
`windows-latest`. Every one of the eleven required tests lives in a
`#![cfg(unix)]` binary (`wrap_watchdog_timeout`, `sequence_schema`,
`wrap_opencode`, `compose_schema_cli`, `composition_outputs`), so Windows cannot
execute them and never could. The gate was doing what the plan forbids —
requiring identical cross-platform counts — and a gate that fails the baseline
itself is not usable on the candidate. Two changes, both tested:

1. **`platformExclusions`** in the expectations: per environment, tests that leg
   cannot run, by name or full identity. An excluded test's absence from that
   leg is reported in its own table (`renderExclusions`) instead of as a
   violation; on every other leg it is still required; and if it ever *does*
   run on the excluding leg, that is a new `stale-exclusion` violation, so the
   list cannot silently outlive the `cfg` that justified it. The timeout-floor
   table prints `excluded` instead of `ABSENT` for those cells.
   `baseline/expectations.json` declares the eleven under `windows-latest`;
   `candidate/expectations.json` is the same file with `enforceTimeoutFloors`
   on, as Phase 1's comment promised.
2. **`--baseline <dir> [--baseline-expect <json>]`**: the comparison the plan's
   fourth bullet needs. Both trees are gated (a violation in either exits 1),
   and every environment present in both is compared *against itself*:
   identities in both are matched and their summed durations set side by side
   with the ratio; candidate-only identities are listed as additions and
   baseline-only ones as removals, each with the seconds they carry. A leg on
   one side only is left out rather than matched against a neighbour — that is
   the cross-platform count comparison in another form.

Requirement → test map, `junit-metrics.test.ts` (46 → 57, all passing):

| Behavior | Test |
|---|---|
| Excluded + absent on its leg → reported apart, no violation | `an_excluded_required_test_absent_on_its_leg_is_reported_apart_from_violations` |
| Exclusion is per-leg; the same absence elsewhere is still `missing-test` | `an_exclusion_is_scoped_to_its_own_leg` |
| Excluded but ran → `stale-exclusion` | `an_exclusion_whose_test_ran_on_that_leg_is_a_stale_exclusion_violation` |
| Bare name and full identity both match; another binary's same-named test does not | `exclusions_match_by_bare_name_or_full_identity` |
| JSON round trip; absent key defaults to `{}` | `expectations_round_trip_platform_exclusions_and_default_them_empty` |
| Passive corpus: every stored `baseline/<run>/` passes the shipped expectations through `main()` | `the_shipped_expectations_accept_every_stored_baseline_run` |
| The shipped exclusions are absent on Windows and present on all three Unix legs, in every stored run | `the_shipped_windows_exclusions_are_absent_on_windows_and_present_elsewhere` |
| Matched within one environment only; additions and removals apart with their seconds | `compare_matches_identities_within_one_environment_only` |
| A real run against itself: everything matched, nothing added or removed | `a_stored_baseline_run_compared_with_itself_matches_everything` |
| `main --baseline` prints the comparison and fails on a violation in the *baseline* tree | `main_with_a_baseline_gates_both_trees_and_prints_the_comparison` |
| `--baseline-expect` without `--baseline` is a usage error; flags parse | `baseline_expect_without_baseline_is_a_usage_error` |

Non-vacuity: the corpus test was run against the pre-change expectations file
first and failed with the same eleven violations the CLI had printed; the
stale-exclusion test fails if the `ran(test)` branch is removed. The comparison
smoke on real data is the PR-run-versus-`main`-run table above: 2466 / 2466 /
2105 / 2466 matched, zero added, zero removed on every leg.

### Consolidated validation — the ledger

`just ci-local` runs lint *and* L1 test by default (`run_lint=1`, `run_test=1`
unless `--lint-only` / `--test-only`), so it was run **once**, without a
preceding `--lint-only` pass. Scope is **73 packages, class=full**: the
branch's `.config/nextest.toml` change against the merge base `a9e88c069` is
41 deleted lines of override blocks, which `affected_scope.py` correctly treats
as a non-comment change to a global path. That is also what CI will select on
push. Tests before lint inside each package, per the stale-binary note.

| Gate | Where | Scope / features | Result | Artifact |
|---|---|---|---|---|
| `just ci-local` | repo root | 73 packages, lint (`--all-targets`, no features) + L1 (declared CI features) | **147 of 147 gates passed, exit 0**, 44 m 16 s (14:15–14:59 UTC); preflight ci-infra self-test included; every package compiled and ran, none skipped | `candidate/local-gates/ci-local.log` |
| `just check-windows` | `claudine/` | `claudine`, `claudine-cli`, `--tests`, `x86_64-pc-windows-gnu` | **exit 0**, 1.3 s warm; 2 unused-import warnings in `wrap_basics.rs` (Phase 1's finding, minus the `compose_caller_file_provenance.rs` one) | `candidate/local-gates/check-windows.log` |
| `just test-l2` | `claudine/` | `claudine-cli` + `claudine-gen`, `terminal-tests` | **236 of 237, exit 100** — the one failure is `level2_typed_error_render_capture::level2_initialize_proxy_block_auto_detects_osc8_in_wezterm`, the same Atuin/WezTerm host condition Phases 4–7 disclosed (`claudine_rc:<code>` exit marker not seen in 30 s; the capture sits behind the Atuin prompt). The recipe aborts before `claudine-gen`, so its three L2 tests were run separately: **3 passed** (`just-test-l2-claudine-gen.log`) | `candidate/local-gates/just-test-l2.log` |
| `just test-l3` | `claudine/` | WezTerm + cliclick keystroke injection | **pending** — not run: L3 injects OS keystrokes into a real window and steals focus on the operator's desktop; forbidden from a non-interactive session | — |
| `just test-real` | `claudine/` | `CLAUDINE_CONTRACT_REAL=1`, `real-tests` | **pending, host condition** — Phase 6 ran it: 4 of 5 fail `Unauthorized` because the provider CLI is not authenticated on this host, identically under the retired `cargo test` route; re-running reproduces a credential state, not code | `log.md` § Phase 6 |
| `npx tsx --test junit-metrics.test.ts` | fix directory | — | **57 passed, 0 failed** | — |
| `junit-metrics.ts` over `baseline/34173378609` and `baseline/34159725015` | fix directory | shipped `expectations.json` | **exit 0, exit 0** | `baseline/<run>/junit-metrics.txt` |
| `attribution.ts --budgets` | fix directory | `budgets-pending.json` at one run per leg | **exit 1, as designed** — four `missing-leg` refusals | — |
| `just test` | `sniff/` | `sniff`, `sniff-cli` L1 | **2599 passed (81 slow), 23 skipped, exit 0**, 80.0 s | — |
| `just lint` | `sniff/` | — | **exit 0** | — |

`sniff` is included because the session was started in that area; no `sniff` file was read or written by this phase, and `ci-local` had already gated both `sniff` packages inside the 147.

Not credited from earlier phases: nothing. Phase 8's `just test` /
`just test-rendezvous` / L2 PTY records remain valid under their reuse rule
(no Rust change since), but this phase's bullet asks for one consolidated
validation and `ci-local` is it; L2 was re-run rather than credited because
it is a two-minute recipe and the ledger is cleaner with a same-day row.

### The PR — handed off, not opened

Everything an operator needs is in [`candidate/README.md`](candidate/README.md)
and [`candidate/pr-body.md`](candidate/pr-body.md). Why it is a handoff:

1. **The branch conflicts with `main`.** `git merge-tree --write-tree
   origin/main HEAD` reports thirteen conflicting files, twenty-one hunks:
   eight `claudine/cli/tests` files that both PR #70 and Phases 4–5 rewrote
   (`common/mod.rs`, `compose_schema_cli.rs`, `composition_outputs.rs`,
   `handle_deadline.rs`, `inline_compose_cli.rs`, `loop_cli.rs`,
   `sequence_groups.rs`, `sequence_prompt_property.rs`); `claudine/justfile`,
   where Phase 6 moved `test-real` onto nextest and `main` added a
   `real_inline_write_grant` invocation to the old `cargo test` form;
   `claudine/lib/src/diagnostics/registry.rs`;
   `claudine/docs/providers/dispatch-inventory.json`;
   `.claude/skills/claudine/timeline.md`; and the completed
   `2026-09-05-inline-flow-and-validations/spec.md`. Resolving a merge writes
   the index and a merge commit — both outside this session's remit — and the
   resolved tree has to be re-validated in `claudine/` before it is pushed,
   because it is a different test population from the one Phases 4–8 measured.
2. **Pushing is outward-facing and the commits must be signed.** The 36 local
   commits are already signed; the Phase 9 working tree is not yet committed
   and the separate commit process owns that.
3. **A PR opened before the merge would be red and unmergeable** on arrival,
   which produces intervening failures the plan would then require recording
   for no evidentiary gain.

The first candidate run on every leg is for cross-platform correctness review;
three consecutive green candidate runs per leg follow from normal CI, with
extra runs requested only for missing samples.

### Budgets — still refused, for a smaller reason

Phase 3 refused to derive budgets because the CI baseline did not exist. It now
exists at **one run of three** per leg, and `attribution/budgets-pending.json`
says so (`runsPerLeg` 1 on each leg, `runs: ["34173378609"]`). `deriveBudgets`
still refuses — `missing-leg: declared but carries no measurements` — for two
reasons that stay open: one run is not three, and nothing yet joins a JUnit
identity to a family (`attribution.ts` reads nextest *logs*; the CI evidence is
XML). So there is no budget to compare against, no miss to explain, and no
universal speedup percentage on offer. The local prior remains what Phase 8
recorded: paired 0.64–0.77 for `just test`, which is the library-heavy
population and not the `claudine-cli` `max-threads = 1` leg this baseline
measures.

### Validation checkpoint 9 — not passed

| Requirement | Status |
|---|---|
| Three consecutive green candidate runs per leg | **pending** — no push; handoff written |
| Three consecutive green baseline runs per leg | **1 of 3** — `34173378609`; `main` moved before a second push run at the same source state could exist |
| Failures disclosed | **done for what exists** — none in either stored run; L3 and `real` recorded as pending, not passing |
| Every budget met or its miss explained | **no budget exists** — refusal reproduced, causes named |
| No override, retry, tier change or disabled assertion used | **held** — the only expectation change is the platform-exclusion declaration, which makes the gate stricter (`stale-exclusion`) rather than looser |

### Deliberate choices

- **Collected the PR's own run as a supplementary sample** rather than
  discarding it: same tree, four green legs, and the only noise bracket the
  baseline has. It is labelled `pull_request`, stored beside the `main` run,
  and counted toward nothing.
- **Did not `gh run rerun 34173378609`** to manufacture the two missing
  baseline samples. It is the right mechanism — same SHA, same workflow — but
  it spends CI minutes on a repository whose cache quota is already saturated,
  and the `main` concurrency group cancels in-flight runs, with `34232285291`
  in flight. Operator call; written up in `baseline/README.md`.
- **Did not record `34232285291` as a baseline run.** It is at `6504747e2`,
  after PR #70 changed the `claudine-cli` test population. If it finishes green
  it is a valid sample of *that* source state and the comparison tool handles
  the population difference; it is not a second sample of the predecessor's.
- **Did not re-run `test-real`.** The failure Phase 6 recorded is a credential
  state on this host. Re-running would drive two external provider CLIs to
  reproduce `Unauthorized`; the evidence is pending until the host is
  authenticated, and saying so is the honest row.
- **Left the two `wrap_basics.rs` warnings alone.** Phase 5 was to fix them
  and did not; they are `#[cfg(unix)]` residue, warnings not errors, and a
  test-file edit now would invalidate Phase 8's reuse rule for a cosmetic gain.
  Carried to Phase 10.

### Carried forward

- **Operator:** merge `origin/main` (thirteen conflicts), re-run `just test`
  and `just lint` in `claudine/`, signed commit, push, `gh pr create
  --body-file candidate/pr-body.md`; then either `gh run rerun 34173378609`
  twice for the missing baseline samples or accept that the baseline is one run
  at `444213eb5` plus whatever `main` produces at later source states.
- **Phase 9, resumed once runs exist:** collect with `candidate/README.md`'s
  recipe; the comparison is `junit-metrics.ts candidate/<run> --expect
  candidate/expectations.json --baseline baseline/34173378609 --baseline-expect
  baseline/expectations.json`.
- **Budget derivation** needs a JUnit → family aggregator to fill
  `perLegFamilySummed`; `inventory-reconciler.ts`'s matcher already maps an
  identity to a family, so it is a join, not a new classifier.
- **`34232285291`** at `6504747e2`: collect when complete, label it with its
  source state.
- **Phase 10 skill candidates** (added to Phase 8's three): the
  `platformExclusions` / `stale-exclusion` pattern for any per-leg required-test
  gate; `gh run rerun` as the only way to sample one SHA twice on `main`; a
  non-comment change to `.config/nextest.toml` selects the full 73-package
  workspace in `ci-local` and CI alike.
- The `wrap_basics.rs` unused-import pair.

## Phase 10 — Closure: `results.md`, drift, acceptance sweep

**Outcome: closed for everything this session can answer; the CI half of
AC6 is pending on the operator, exactly as Phase 9 left it.** `results.md`
exists and keeps the three completion claims apart; seven deferred findings
have an owner document; three skill files changed where a workflow claim was
missing or false; two test files gained `#[cfg(unix)]` import gates; every
local gate the plan names was run or credited with its source-state argument;
no gate was weakened.

### Grounding facts re-checked (2026-09-08, 15:10–15:35 UTC)

- **No candidate CI run exists.** `gh run list --branch fix/cli-slow-tests`
  shows nothing after the PR #69 runs at `a9e88c069`; nothing has been pushed.
- **The remote branch is gone.** `git ls-remote --heads origin
  fix/cli-slow-tests` returns nothing — GitHub deleted the head branch when
  PR #69 merged. The next push recreates it; `candidate/README.md` and
  `results.md` say so.
- **`main` is still `6504747e2`** (local and origin agree). Its run
  `34232285291` was still in progress at 15:13 UTC: every `claudine*` native
  L1 leg, lint and check green; `claudine-cli` WSL2 archive in progress; the
  L2 legs queued. Different source state from the baseline; recorded, not
  collected.
- **`git diff origin/main -- .config/nextest.toml`**: 41 deletions, 16
  insertions, zero non-comment insertions — unchanged since Phase 6.
- **The startup-stall spec link the Phase 3 note flagged** resolves in
  `plan.md` (`_completed/…`); the dangling ones were in `spec.md`, which still
  pointed at both archived fixes' pre-`_completed` paths. Repaired there.
- **The Windows toolchain is present**; `just check-windows` warm is 0.9 s.

### What shipped

| Item | Where |
|---|---|
| `results.md` — measurements per leg with the three costs apart, coverage changes with replacement coverage for all sixteen removals, residual findings in three classes, the AC table, the gate ledger, the operator handoff | [`results.md`](results.md) |
| Owner document for the seven deferrals (bench files, `context` render seam, the vacuous ownership-kill test, `real_provider`'s credential path, the failing ignored perf harness, the `max-threads = 1` group, the JUnit → family aggregator) | [`../_unscheduled/test-suite-residuals/spec.md`](../_unscheduled/test-suite-residuals/spec.md) |
| Two `#[cfg(unix)]` import gates: `wrap_basics.rs` (`std::fs`, `common::wrap::*`) and `compose_caller_file_provenance.rs` (`write_executable`) | `claudine/cli/tests/` |
| `rust-testing/SKILL.md`: the cross-compile route (mingw vs MSVC, warm-check caveat), narrowing a recipe to one binary, a global-path change selects the whole workspace | `.claude/skills/rust-testing/SKILL.md` |
| `rust-testing/test-suite-audits.md` § Measurement: exact-recipe warm-up, drift bracket beside paired ratio, lldb entry-location counters and the `PATH` shim, per-leg exclusions with stale-exclusion failure, `gh run rerun` for a second sample | `.claude/skills/rust-testing/test-suite-audits.md` |
| `claudine/signal-handling.md`: the Windows console-control row no longer says there is no green runtime run — `34173378609` ran both tests green | `.claude/skills/claudine/signal-handling.md` |
| `spec.md`: five links/paths repointed at `_completed/` | [`spec.md`](spec.md) |

**The third warning.** Phase 9 recorded the `compose_caller_file_provenance.rs`
unused-import warning as gone and only the `wrap_basics.rs` pair remaining.
Phase 10's first warm check, after gating the `wrap_basics.rs` pair, reported
the `compose_caller_file_provenance.rs` one instead. The two warm runs replayed
different subsets of cached diagnostics, so neither was a complete count; the
import was gated the same way and a **cold** check into a fresh
`CARGO_TARGET_DIR` (`phase10-check-windows-cold.log`, 1 m 28 s) is the
definitive count: **zero warnings**. The caveat is now in the skill.

### Requirement → test map

Phase 10 changed no behavior, so it added no test. The two edits are import
gates whose only observable is the Windows target's warning count, and the
gate for that is the cold `just check-windows` (0 warnings, exit 0) plus the
unchanged Unix result (`just test` 6873 passed — the gated imports are still
in scope for every `#[cfg(unix)]` case, or the binaries would not compile).
The plan's Phase 10 bullets are document deliverables; their "test" is the
acceptance table in `results.md`, each row pointing at the artifact that
proves it.

### Gate ledger

Tests before lint, per the stale-binary note. Logs under
`candidate/local-gates/phase10-*.log`.

| Gate | Where | Result | Run / credited |
|---|---|---|---|
| `just test` | `claudine/` | **6873 passed / 9 skipped, exit 0**, 35.7 s runner elapsed (1 m 46 s wall with the `wrap_basics` rebuild) | run, after the first edit |
| `just test-leaks claudine` | repo root | **7146 passed / 11 skipped (1 slow), `no leaked processes detected`, exit 0**, 39.8 s / 3 m 08 s | run, after both edits |
| `just check-windows` | `claudine/` | **exit 0, 0 warnings** warm (0.9 s) and cold (1 m 28 s, fresh target dir) | run |
| `just lint` | `claudine/` | **exit 0, 0 warnings**, 5 m 03 s wall (overlapping the cold check) | run, after the tests |
| `just doctest` | `claudine/` | **exit 0** — 20 + 3 + 2 passed, 7 ignored; `claudine-cli` skipped (no lib target) | run |
| `just test` / `just lint` | `sniff/` | **2599 passed / 23 skipped, exit 0** (52.4 s); **exit 0** | run — the session's starting area; no `sniff` file changed |
| `just test-rendezvous` | `claudine/` | 273 / 2, ×11 | credited, Phase 8; also inside the leak sweep's 7146 |
| `just test-l2` | `claudine/` | 236 / 237 + `claudine-gen` 3 / 3 | credited, Phase 9 (same day, same source); the survivor is the Atuin/WezTerm host condition |
| `just bench` | `claudine/` | exit 0 (`BENCH_YES=1`) | credited, Phase 6; no bench input changed |
| `just ci-local` | repo root | 147 / 147 | credited, Phase 9; the tree differs by two import gates |
| `junit-metrics.test.ts` | fix directory | 57 passed | credited, Phase 9; script unchanged |
| `just test-l3` | `claudine/` | — | **pending**, focus-stealing tier |
| `just test-real` | `claudine/` | 1 / 4 `Unauthorized` | **pending**, host credential state |
| `git diff origin/main -- .config/nextest.toml` | repo root | 41 − / 16 + (all comments) | run |

### Deliberate choices

- **Fixed the import warnings instead of deferring them.** Two lines each,
  zero behavior, and the alternative was a deferral entry for a warning the
  area's own gate emits. It cost the reuse rule for the `claudine-cli` suite,
  which the plan's "required final coverage" bullet was going to re-run anyway.
- **Ran a cold Windows check.** A warm check's warning count proved unreliable
  across Phases 9 and 10; the cold count is the only one worth writing down.
- **Credited rather than re-ran `test-l2`, `bench`, `test-rendezvous`.** None
  of their inputs changed since the run credited; the rendezvous population
  also ran again inside the leak sweep. Re-running `test-l2` would have
  reproduced the Atuin host condition for no new evidence.
- **One owner document for seven deferrals** rather than seven. They share an
  origin and a reader; each has its own section, evidence, reason and closing
  criteria, which is what AC4 asks for.
- **Skill edits stayed at the workflow level.** No Phase 8/9 number went into
  a skill; what went in is the method (exact-recipe warm-up, paired ratio,
  entry-location counters, per-leg exclusions) and the two facts that changed
  the predecessor's conclusion (mingw route; global-path scope).
- **Did not touch `deferred-performance.md`.** Item 1 is still one run of
  three; Phase 9's addendum is current.
- **Did not gzip the Phase 10 logs.** Phase 9 stored `ci-local.log` (3.7 MB)
  uncompressed under the same directory; matched that precedent.

### Validation checkpoint 10 — passed for what this session can answer

| Requirement | Status |
|---|---|
| All seven ACs answered with evidence or a linked deferral | **yes** — AC1–5, AC7 verified; AC6 verified locally, **pending on CI** with the cause named |
| `results.md` keeps the three completion claims separate | **yes** — first table |
| No gate weakened to close a criterion | **yes** — the nextest diff is Phase 6's; no override, retry, tier change, `#[ignore]` or dropped assertion anywhere in Phases 4–10 |

### Carried forward — operator

Unchanged from Phase 9, plus one fact: merge `origin/main` (thirteen
conflicts), re-run `just test` and `just lint` in `claudine/` on the merged
tree, signed commit, push (**the remote branch has to be recreated**),
`gh pr create --base main --body-file candidate/pr-body.md`; then three green
candidate runs per leg collected with `candidate/README.md`'s recipe, and
either `gh run rerun 34173378609` twice or an accepted one-run baseline. The
budget path needs residual 7 (the aggregator) before `deriveBudgets` can
produce a table. The spec directory stays where it is; archiving to
`_completed` is a separate step.

## 2026-09-08 — analysis tools moved to the shared `tools/test-audit` package

Done by the darkmatter fix's Phase 1A (`darkmatter/fixes/2026-09-07-faster-darkmatter-tests`, plan § Phase 1A), not by this fix.

- `junit-metrics.ts`, `inventory-reconciler.ts`, `attribution.ts`, `measurement.ts`, and `measurement-runner.ts` are now thin wrappers that forward to `tools/test-audit` (`junit`, `reconcile`, `attribute`, `measure`, `measure run`) with `audit.config.json` beside them, which carries what the scripts hard-coded (package roots, the four legs and their one gated cell, the eleven required timeout-shaped tests, the timeout floors). Every recorded invocation in this log still works unchanged.
- Their `*.test.ts` files moved into `tools/test-audit/tests/` (ported to vitest) together with `*-claudine-compat.test.ts` replays over the preserved inputs here: the JUnit gate reproduces `baseline/34173378609/junit-metrics.txt`; `attribute` reproduces `attribution.md` (6,861 results, 561.00 s summed, 36.05 s elapsed on `baseline/local-gates/just-test.log`); `measure report` reproduces `measurement/report.md` and `report.gate.txt`; the reconciler reproduces the family index (7,400 runner identities) and, against the preserved baseline worktree `/tmp/rb-baseline-9fc5151a0`, the source column (4,167 / 21 / 2,761 / 52 / 158 / 24 / 88 / 184) with zero exclusion violations.
- `sentinels.ts` and `attribution/launch-cwd-probe.ts` stay local (lldb symbol lists and the `git` shim are Claudine-specific).
- Finding for this fix's follow-up: the `enumeration/` captures describe 9fc5151a0, and the working tree has since gained tests (Phases 4–7). `npx tsx inventory-reconciler.ts` on the live tree therefore reports those as `undeclared-exclusion` (e.g. `claudine-cli :: the_spawn_gate_reads_a_real_population_and_still_finds_a_planted_site`). That is the gate working; re-run `test-audit capture --config audit.config.json` at the committed candidate revision before the next inventory checkpoint.
- Tool version for every report from here on: `@rusty-biscuit/test-audit@0.1.0`.

## Implementation of Review Findings #1

> **started at:** 2026-09-09T10:53:47-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Users/ken/.claudine/worktrees/rusty-biscuit/fix-cli-slow-tests/claudine/fixes/2026-09-07-faster-claudine-tests/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review carries three numbered findings plus one verification-level defect
  raised in the requirement table, which its own **Closure Criteria** section
  enumerates as four items; those four are the work units below
        - **C1** — reclassify the four PTY-only binaries as Level 1, migrate all
          19 tests into the `CliProcessFixture` contract, and make
          `spawn_site_guard` classify by resource boundary rather than filename
        - **C2** — add real Level-2 proof for the wrapper-summary rendering
          claim, or narrow the claim to what the Level-1 assertions establish
        - **C3** — isolate the WezTerm pane's shell startup from host
          configuration so the canonical `just test-l2` run is green
        - **C4** — integrate `main`, collect the consecutive baseline/candidate
          CI runs on all four legs, aggregate JUnit into families, ratify budgets
- orchestration is serial: one subagent per work unit, each running the
  claudine area's own `just test` / `just lint` gates before reporting back
- starting the work on 'C1 — reclassify and migrate the four PTY-only binaries to Level 1' at 10:55:41-07:00
        - reclassified the four `expectrl` binaries from L2 to L1 with `git mv`,
          so history follows: `level2_pty_tests.rs` → `level1_pty_wrapper_summary.rs`,
          `level2_schema_prompt_pty.rs` → `level1_schema_prompt_pty.rs`,
          `level2_provided_partial_file_pty.rs` → `level1_provided_partial_file_pty.rs`,
          `level2_dry_run_pty.rs` → `level1_dry_run_pty.rs`
        - all 19 test functions renamed `level2_pty_…` → `level1_pty_…`; every
          `require_level!(Level::L2, …)` is now `Level::L1`; each `//!` header
          gained a `## Tier` section stating why a bare PTY is Level 1, and
          `level1_dry_run_pty.rs` keeps its cross-reference to
          `level2_dry_run_approval_capture.rs` as the emulator-level complement
        - `expectrl` was already an unconditional dev-dependency
          (`claudine/cli/Cargo.toml:94`), so the four
          `required-features = ["terminal-tests"]` blocks were removable at no
          cost — which also means these 19 tests were **not previously compiled
          at all** on the local `just test` route, only on `just test-l2`
        - every Claudine child now comes from `CliProcessFixture`:
          `command_std()` for the ordinary shape, and
          `command_builder().fake_only_path().build_std()` for
          `level1_pty_wrapper_summary` (whose original `PATH` was the fake bin
          alone; the escape carries the call-site comment the contract requires).
          Four hand-rolled `Command` duplicates in the schema file collapsed into
          one `claudine_command(fixture, args)` helper
        - the migration exposed a latent capture race: `level1_dry_run_pty`'s
          parity test waited on the label `"Blacklist and stop"`, which can match
          mid-line, so under full-suite parallel load the two captures differed by
          a truncated final row. It now waits on that option's tail,
          `"(persists to blacklist)"`. No assertion was weakened — the comparison
          is still byte-identical after ANSI stripping
        - `spawn_site_guard.rs` now classifies by **resource, not filename**:
          `EXCLUDED_PREFIXES` is replaced by `exemption(relative, source)`, which
          exempts a file only when it names a `biscuit-test-harness` emulator
          constructor (`TmuxHarness`/`WezTermHarness`/`KittyHarness`/`AppleTerminalHarness`/`shared_or_spawn`)
          in sanitized executable code, or is the opt-in `real_` tier — the one
          surviving prefix rule, because an authenticated provider account has no
          source-visible constructor. Opening a PTY is explicitly not a resource.
          The governed population grew from 90 files to **95**
        - a new failure mode closes the blind spot in the other direction:
          `a_terminal_tier_name_must_match_the_resource_the_file_owns` fails any
          `level2_`/`level3_`-named file that constructs no emulator session, and
          any file that constructs one *without* the prefix (which would let
          `just test` spawn real panes)
        - non-vacuity proven by two reverted experiments — (a) restoring one raw
          `cargo_bin` spawn in `level1_pty_wrapper_summary.rs` produced
          `FAIL … l1_tests_spawn_claudine_through_the_fixture_builder` naming
          `level1_pty_wrapper_summary.rs:46`, a site the old filename rule made
          invisible; (b) renaming `level1_dry_run_pty.rs` back to `level2_…`
          produced `FAIL … a_terminal_tier_name_must_match_the_resource_the_file_owns`.
          Both restored and re-run green
        - the 19 tests were **counted, not assumed**, in the L1 route: an isolated
          run reports `Summary [4.720s] 19 tests run: 19 passed, 0 skipped`, and
          all 19 appear as `PASS` inside the full `just test-cli` output
        - documents reconciled: family `cli-l2-pty` → `cli-l1-pty-interactive` in
          `families.json`, moved into the "L1 integration binaries" block of
          `inventory.md`, both rows corrected in `attribution.md`, and in
          `results.md` the AC2 census (90 → 95, with the reason) plus the
          `just test-l2` count 237 → 218 marked **stale / not re-run**
        - `.config/nextest.toml` was deliberately left alone and re-verified:
          218 `level2_`-named tests remain in `claudine-cli`, so neither the
          slow-timeout filter nor the CI retries-0 filter is stale.
          `git status -- .config/nextest.toml` is empty (AC5, AC7)
        - `enumeration/` was **not** regenerated — it is deliberately frozen at
          `9fc5151a0`. `inventory-reconciler.ts` therefore reports
          `[stale-family] cli-l1-pty-interactive matches no identity` and
          `[inventory-drift] inventory says 19, the captures say 0`. It was
          already exiting 1 before this work: 54 of its 75 `undeclared-exclusion`
          entries name tests untouched here, because the branch has moved past the
          capture revision
        - **finding, not fixed here:** the sibling guards
          `sniff/cli/tests/spawn_site_guard.rs` and
          `darkmatter/cli/tests/spawn_site_guard.rs` still classify by filename
          and carry the same blind spot. It is latent in both — neither area has a
          `level2_*` file that uses `expectrl` today — and outside this fix's
          package scope
        - **pre-existing branch failures disclosed, not absorbed.**
          `just test` is 6880 passed / 17 failed / 1 timed out and `just test-cli`
          is 2515 passed / 7 failed / 1 timed out on this tree. None is in a file
          this work touched: the only edits to shared test code
          (`common/mod.rs`, `common/pty.rs`) are comment-only, verified by
          reading the diff. The set is 5 × `loop_control::target_launch::tests::*`,
          `propagated_context_fixtures::isolated_fixture_can_opt_in_to_provider_memory_discovery`
          (fails in isolation), `spawn_inventory::production_spawn_inventory_is_complete_and_governed`
          (line-number drift in production sources from commit `f0aaa4832`),
          10 × `claudine-gen::drift`/`generate_ux` (the known archived-baseline
          break), and a `wrap_sigint` timeout that reproduces in isolation. This
          is a regression in the branch since the fix's Phase 10 green run, and
          it is a finding for the operator rather than something this cycle
          can close
- work completed for 'C1 — reclassify and migrate the four PTY-only binaries to Level 1' at 11:26:10-07:00
- starting the work on 'C2 — reconcile the wrapper-summary rendering claim with its evidence' at 11:26:20-07:00
        - route chosen: **narrow the claim**, which the review permits as an
          alternative to adding Level-2 proof. The decision was made from a
          negative search, not from convenience
        - every `level2_*`/`level3_*` binary in `claudine/cli/tests`,
          `claudine/lib/tests` and `claudine/gen/tests` was read; none asserts
          the wrapper header row that `claudine/cli/src/output/mod.rs`'s
          `log_wrapper_header` emits. The two near-misses are
          `level2_perf_capture.rs` (prints the row, but its own comment records
          that the headline scrolls out of the viewport and every assertion is on
          the perf tree) and `level2_stalled_generation_capture.rs` (the only L2
          test on the bare wrap path, asserting only the `Agent Error` block).
          `level2_dry_run_metadata_capture.rs` captures a red `YOLO` cell in the
          `--dry-run` *metadata table* — a different surface
        - the closest genuine evidence is component-level, not surface-level:
          `biscuit-terminal-cli::level2_prose_styling` decodes `Bold`, `Italic`,
          `FgRgb`, `BgRgb` and `Dim` from real WezTerm and Kitty captures, which
          proves the `Prose` renderer the badges are built from — but not this
          row's composition, spacing, glyph width, or truncation
        - a second overstatement surfaced while checking: of the old test's five
          assertions only **one** (`YOLO`) is a badge at all. `INTERACTIVE` is a
          row of the environment-variables table, and the `-n` flag actively
          suppresses the Interactive badge. The name overstated the test on two
          axes, not one
        - renamed `level1_pty_wrapper_summary_shows_badges` →
          `level1_pty_wrapper_summary_text_precedes_child_output`. All five
          `expect` calls are unchanged and in the same order: this is a statement
          change, not a coverage change, and the test count is 2523 before and
          after
        - the module `//!` docs gained a "What these assertions establish, and
          what they do not" section — textual content and ordering in the child's
          byte stream; explicitly nothing about glyph width, SGR styling, or badge
          row layout — naming where the neighboring L2 evidence actually lives
        - `inventory.md`'s `cli-l1-pty-interactive` row now states the family's
          claims as textual and records the "visible as rendered terminal UI"
          claim as **withdrawn**; its Disposition carries the full negative search
          result and the explicit decision to record a gap rather than manufacture
          coverage. `results.md` gained a **Narrowed claims** paragraph naming the
          rename and "replacement coverage: none, by decision", which is what the
          spec's "record every assertion or test-population change and its
          replacement coverage" asks for
        - no new L2 capture binary was written. The review permits narrowing, this
          is a test-performance fix, and ~2 s of L2 cost for a surface with no
          identified regression pressure is against the repo's Rule 2. The gap is
          recorded as a gap in both documents so a reader sees an honest hole
          rather than phantom coverage
        - gates: `just test-cli` reproduces the baseline **exactly** — 2523 run,
          2515 passed, 7 failed, 1 timed out, 9 skipped, the same eight
          pre-existing identities and zero added failures; `just lint` exit 0 with
          zero warnings; `git status -- .config/nextest.toml` empty
- work completed for 'C2 — reconcile the wrapper-summary rendering claim with its evidence' at 11:36:05-07:00
- starting the work on 'C3 — isolate the WezTerm pane's shell startup from host configuration' at 11:36:20-07:00
        - root cause **confirmed on this host, not assumed**.
          `biscuit-test-harness/src/lib.rs::configure_login_shell` built
          `bash -l -c '… exec "$0" -i' bash`. The inner `exec "$0" -i` is a
          *non-login interactive* bash, so it reads `~/.bashrc`, whose line 3 on
          this host is `eval "$(atuin init bash)"`. Atuin's first-run picker
          rendered into the pane and swallowed the line the harness sent, so
          `; echo claudine_rc:$?` never ran
        - the two shells serve unrelated purposes and need **opposite** startup
          files: the outer `-l` is the only source of the host `PATH` when a
          terminal GUI is launched from the desktop (on this host
          `~/.bash_profile` supplies `cargo env` and `~/.local/bin`), so it stays;
          the inner `-i` exists only so the harness can see a prompt and send
          lines, needs nothing from the rc file, and the rc file is exactly where
          Atuin, starship, fzf and zoxide install themselves
        - shipped in `biscuit-test-harness/src/lib.rs`:
          `interactive_rc_suppression_flags(shell)` (`bash` ⇒ `--norc`,
          `zsh` ⇒ `-f`, otherwise empty, matched on the shell's *file name* so an
          absolute `$SHELL` still resolves) and `login_shell_script(shell, augment_path)`,
          which always prefixes `unset ENV BASH_ENV;` because POSIX
          `sh`/`dash`/`ksh` have no rc flag and source whatever `$ENV` names
        - `configure_login_shell` collapsed to one code path. The old
          `else { cmd.arg("-l") }` branch produced a *single* interactive-login
          shell and was contaminated on any host whose `.bash_profile` sources
          `.bashrc`; it now gets the same two-stage treatment
        - no custom `PS1`. Rc suppression leaves bash on its stock `\s-\v\$`
          (`bash-3.2$`), which `looks_like_prompt` already accepts; a synthetic
          prompt would add a fresh collision surface against captured content for
          no gain. `stock_prompts_of_rc_suppressed_shells_are_recognized` pins
          that coupling so the two cannot drift apart
        - six non-vacuous unit tests added, each asserting the **constructed
          argv** rather than an outcome:
          `login_shell_script_suppresses_interactive_rc_for_bash` / `..._for_zsh`,
          `login_shell_script_clears_env_for_shells_without_an_rc_flag`,
          `interactive_rc_suppression_flags_match_on_the_shell_file_name`,
          `login_shell_script_prepends_the_bin_dir_after_login_startup`, and the
          prompt-coupling test above
        - **the canonical gate is green.** `just test-l2` in the `claudine` area:
          `claudine-cli` `Summary [51.043s] 218 tests run: 218 passed, 2536 skipped`
          and `claudine-gen` `Summary [2.585s] 3 tests run: 3 passed, 155 skipped`,
          **EXIT=0**, no `FAIL` and no `error:` lines. The previously surviving
          test appears inline as
          `PASS [3.252s] (207/218) … level2_initialize_proxy_block_auto_detects_osc8_in_wezterm`
        - before/after on the focused reproduction: `FAIL [34.089s]` with the
          Atuin picker in the captured frame → `PASS [2.903s]`
        - blast radius checked beyond the required gates, because
          `biscuit-test-harness` is shared: `biscuit-terminal` 76 + 2 passed,
          `worktree` 5 passed, `tree-hugger` 3 passed, `biscuit-icon` 11 passed,
          `biscuit-tui` 21 passed, all exit 0. `biscuit-test-harness`'s own
          `just test` is 103 passed and `just lint` exit 0
        - darkmatter's L2 showed a ~26 s stall failing a **different identity on
          each run**; A/B'd by temporarily reverting the rc suppression in place —
          `level2_list_center_alignment_indents_more_than_left` failed with rc
          files loaded, `level2_cli_align_lists_broadcast_indents_in_real_terminal`
          failed with them suppressed, and a re-run with the fix was 10/10. A
          pre-existing wandering flake on that branch, not caused here; the file
          was restored exactly and every gate re-run afterwards
        - **host trap re-confirmed and worked around, not papered over:** an agent
          session Claudine launched with `--model opus` exports ambient
          `MODEL=opus`, which fails
          `level2_lifecycle_equivalence_ac9_context_facets_match_direct_run`
          (`ctx.model=opus` vs the probe's pinned value). Run L2 as
          `env -u MODEL just test-l2`. No test or config was changed to make it pass
        - two unadvertised wins fall out of the same change: an rc-installed
          `precmd` title hook can no longer clobber `wezterm cli set-tab-title`'s
          harness tag (which L3's `AXRaise` matches on), and prompt generators
          that query terminal colors on each redraw can no longer perturb
          `level2_terminal_osc_wezterm`'s exactly-one-OSC-10 count
        - docs updated alongside the behavior: `configure_login_shell`'s docblock,
          `PROMPT_TERMINATORS`' now-stale claim that the harness always inherits
          host dotfiles, the `spawn_shell` notes in `wezterm.rs` and `kitty.rs`,
          `biscuit-test-harness/README.md`, and
          `.claude/skills/biscuit-test-harness/SKILL.md`
        - **finding, not fixed here:** the tmux and Apple Terminal backends still
          run a *single* `bash -l`. On this host that reads `~/.bash_profile`
          only — no Atuin — which is exactly why 217 of 218 passed while the one
          WezTerm test failed. The hazard is latent rather than absent: on a host
          whose `.bash_profile` sources `.bashrc` (the Debian/Fedora skeleton
          default) tmux would inherit the same contamination. It is a ~10-line
          change reusing `login_shell_script`, but it crosses five package areas
          whose suites are being refactored concurrently in this worktree, so a
          regression could not have been attributed. Recorded for follow-up
        - **second finding, not fixed here:** `assert_no_sgr_red` in
          `biscuit-terminal/cli/tests/level2_prose_styling.rs:906` has a
          prompt-terminator `return` that now reliably fires and shortens its
          10-line negative-assertion window. It already fired on this host before
          the change (the stock prompt was already `bash-5.3$`), so this work did
          not alter it, and the suite is green. It belongs to `biscuit-terminal`
- work completed for 'C3 — isolate the WezTerm pane's shell startup from host configuration' at 12:32:40-07:00
- starting the work on 'C4 — CI performance contract: aggregation, evidence, budgets' at 12:32:50-07:00
        - split on what this session can actually close: the JUnit-to-family
          aggregation is code and is implementable now; the consecutive candidate
          CI runs are operator-gated and are the deferral
        - **built:** `test-audit attribute aggregate <run-dir>… --config <cfg>
          [--families <json>] [--out <json>] [--provenance ci|local] [--headroom <ratio>]`
          in `tools/test-audit/src/attribute/aggregate.ts`, with the subcommand
          wired in `src/attribute/command.ts`. It walks
          `<run-id>/<env>/<tier>/<package>.xml` staging trees, projects each cell
          through the existing `invocationFromJunit`, and resolves every
          `<testcase>` through the reconciler's existing `familiesMatching` — a
          join, not a second classifier, which is what residual 7 said was missing
        - it lives in the shared `tools/test-audit` package rather than the fix
          directory, so darkmatter's and sniff's parallel fixes inherit it. No new
          wrapper was needed: `attribution.ts` forwards positionals unchanged, so
          `npx tsx attribution.ts aggregate baseline/34173378609` already works
        - it is a gate, not a report — exit 0 clean, 1 on violation, 2 on usage,
          matching every other tool in the package. Disqualifying classes, each
          with a named test: red run (non-zero manifest `exit_code`, or a failed
          `<testcase>`), malformed or missing report, missing manifest, a manifest
          record naming another environment, a declared leg absent, a leg
          declaring no cells, a cell with no manifest record,
          `report_present=false`, an XML named but absent, a count mismatch, a
          duplicate identity, an identity claimed by zero families, an identity
          claimed by two (counted by **neither**, as the reconciler already does),
          and the same run id supplied twice
        - real-artifact result against `baseline/34173378609/`: the join is
          **total** — 0 unclaimed, 0 double-claimed, on all four legs — and the
          per-leg summed durations reproduce `results.md` § CI baseline to 0.1 s
          (324.7 / 753.5 / 355.8 / 692.2 s)
        - **the refusal narrowed; it did not disappear.** `deriveBudgets` moved
          from `[missing-leg] declared but carries no measurements` to
          `[insufficient-runs] 1 green run(s); 3 consecutive are required` on all
          four legs. That is the correct end state at one stored run, and it is
          the evidence that the join works while the budget still refuses
        - `runsPerLeg` was not raised and `perLegFamilySummed` was not hand-filled
          in `budgets-pending.json`; the recorded refusal reproduces byte-identical
        - the `cli-l2-pty` → `cli-l1-pty-interactive` rename from C1 is handled
          without a special case: the renamed family's four suites are simply
          absent from the stored L1 cell, so it carries no sample and
          `deriveBudgets`'s "fewer samples than runs" rule catches it. There is an
          explicit test for that
        - 36 new vitest tests (28 unit + 8 real-artifact compat). Non-vacuity by
          neuter, six detectors disabled one at a time with the file restored
          between each; `sha256(aggregate.ts)` is
          `35e2a2b4e0ff72f6e95acc0f27c5e5d76d32ddc66d33270f77b7cdf43ec9aae6`
          before, after every restore, and at the end
        - `--provenance local` exists so a local `target/nextest/ci-reports` tree
          can be aggregated and then refused downstream by
          `local-derived-budget`, rather than quietly becoming a target
        - documents reconciled: `tools/test-audit/README.md`,
          `claudine/fixes/_unscheduled/test-suite-residuals/spec.md` §7 rewritten
          as a **half-closed** deferral, `inventory.md` § Budgets, `attribution.md`
          (procedure step 1 plus a "Step 1 shipped" section), and `results.md`'s
          AC6 row — whose *reason* for being pending changed while its **status did
          not**
        - **regression surfaced, owned below:** `tools/test-audit`'s own
          `just test` is now 197 passed / 3 failed, and all three failures trace to
          C1's `families.json` rename replaying against the frozen `enumeration/`
          captures. Proof, same logs and same code with two family files —
          `HEAD families.json: 0 violation(s)` vs
          `working-tree families.json: 19 violation(s)`, exactly the 19 identities
          C1 moved. The shared package was green at `HEAD` and is red on this tree,
          so this cycle owns it
        - **not done and not fakeable from here:** the `main` merge (13 conflicting
          files), the OpenPGP-signed commit, the push (the remote branch was
          auto-deleted at PR #69's merge), the pull request, and three consecutive
          green Actions runs per leg across four environments. No budget can be
          ratified until that evidence exists
- work completed for 'C4 — CI performance contract: aggregation, evidence, budgets' at 12:48:20-07:00
        - the aggregation half is **complete**; the evidence and ratification
          halves are **deferred**, with the reason recorded in the closing section
- starting the work on 'C1-followup — regenerate the enumeration captures and reconcile the inventory' at 12:48:35-07:00
        - the review's Closure Criterion 1 ends "regenerate inventory and
          measurement evidence", which C1 deliberately left alone; C4 then showed
          the cost of leaving it, so it is picked up here as its own unit
