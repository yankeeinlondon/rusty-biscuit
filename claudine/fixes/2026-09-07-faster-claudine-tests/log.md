---
fix: 2026-09-07-faster-claudine-tests
phase: 3
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
