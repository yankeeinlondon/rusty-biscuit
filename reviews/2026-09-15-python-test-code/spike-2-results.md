---
title: "Spike 2 results — plan-structure problems keep the receipt vocabulary"
created: 2026-09-15
status: complete
source: reviews/2026-09-15-python-test-code/review.md §5.1
decides: whether to add PLAN_REJECTIONS and subdivide malformed-receipt
decision: "No fourth vocabulary. One measured collision, and it needed no new code — the three colliding messages were already distinct and only the test matched their shared tail; separately, the code a plan-structure problem carries when it reaches CI is assigned by local_evidence.py, not schema.py, so PLAN_REJECTIONS could not change what any consumer sees from inside schema.py."
---

# Spike 2 results — should plan-structure problems have their own vocabulary?

## Ruling against the decision rule

Measured collision count is **1**, which lands in the plan's *1–2 collisions* band:
"apply only to the colliding rules; record that the prose is precise enough
elsewhere and stop."

Applying it to the one colliding rule set turned out to need no rejection code at
all. The three rules that collide (`schema.py:611`, `:633`, `:706`) already emit
three textually distinct messages — `area 'claudine' has no selection reason`,
`package 'claudine' has no selection reason`,
`cell claudine/ubuntu-latest/L1 has no selection reason`. The collision was in the
test, which matched the shared tail `"selection reason"` for all three. Asserting
the exact text per rule removes it.

Step 2 independently forecloses the larger change: **the code a plan-structure
problem carries when it reaches CI is assigned by
`scripts/ci/local_evidence.py:261`, not by `schema.py`.** A fourth vocabulary in
`schema.py` therefore could not change what any consumer sees without editing a
production file this spike does not own, and doing it in `schema.py` alone would
emit double-coded rejections. One test also keys on the literal prefix for a
plan-structure problem (`scripts/ci/test_resolved_plan.py:543`) and would go
silently vacuous.

`REJECTIONS`, `SCOPE_REJECTIONS`, `BUILD_REJECTIONS` and
`.github/ci/schemas/contract.json` are unchanged.

## Step 0 — inventory

The plan's figures come from a `problems.append(...)` regex, which counts
receipt-side rules and misses every message returned from a helper or built in a
comprehension. Reproduced for the record: that regex finds 53 literals,
`malformed-receipt` ×45. The real surface reachable from `validate_resolved_plan`
is **55 emission sites**, obtained by walking the AST of `validate_resolved_plan`
and the seven helpers it reaches (`_build_records`, `_build_consumers`,
`_build_identity`, `_build_references`, `_dependent_seam`, `_cell_consistency`,
plus the shared `_keys`, `_member`, `_sha`, `_str_list`). `_counts` is receipt-only
and excluded.

By code: **46 of the 55 are always `malformed-receipt`** — 43 hard-coded plus the
three `_keys` templates, whose `code` parameter defaults to `malformed-receipt` and
is left at the default at all eight plan-side call sites. `_member` supplies a
47th template used both ways: `malformed-receipt` at four plan call sites
(`change_class`, `execution`, `origin`, `state`) and `unknown-environment` /
`unknown-gate` at four others. The remainder: `unknown-package` ×5,
`unknown-environment` ×1, `unknown-schema-version` ×1, `missing-receipt` ×1.

"Pinned by" was measured, not read: a line-tagged copy of `schema.py` ran beside
the real one through the whole of `test_schema.py`, recording which emission line
produced each problem per test; a site counts as pinned when some string literal
in that test's body is a substring of a message the site actually produced. The
two substring assertions elsewhere in the corpus
(`test_affected_scope.py:2482`, `:2585`) were checked by hand. Every other caller
of `validate_resolved_plan` in the suites asserts `== []` and pins no rule.

| schema.py | function | code | rule (message template) | pinned by |
|---:|---|---|---|---|
| 538 | `_keys` | «code param» | `{…} must be an object` | **nothing** |
| 540 | `_keys` | «code param» | `{…} is missing required field '{…}'` | `test_a_missing_field_names_the_field`, `test_every_unhashed_identity_field_is_required`, `test_affected_scope.py:2482` |
| 545 | `_keys` | «code param» | `{…} has unknown field '{…}'` | `test_an_unknown_field_is_rejected` |
| 555 | `_member` | «code param» | `{…} is {…}, expected one of {…}` | `test_unknown_environment_and_gate_use_their_own_codes` |
| 561 | `_sha` | `malformed-receipt` | `{…} is not a full Git object ID: {…}` | **nothing** |
| 566 | `_str_list` | `malformed-receipt` | `{…} must be a list of strings` | **nothing** |
| 570 | `_str_list` | `malformed-receipt` | `{…} contains unknown value {…}` | `test_blanket_all_targets_is_not_a_target_kind` |
| 590 | `validate_resolved_plan` | `unknown-schema-version` | `resolved plan is version {…}, this tool writes {…}` | `test_a_future_schema_version_is_rejected_by_code` |
| 611 | `validate_resolved_plan` | `malformed-receipt` | `area {…} has no selection reason` | `test_every_area_package_and_cell_states_a_selection_reason` |
| 622 | `validate_resolved_plan` | `unknown-package` | `package {…} names area {…}, which the plan does not select` | **nothing** |
| 633 | `validate_resolved_plan` | `malformed-receipt` | `package {…} has no selection reason` | `test_every_area_package_and_cell_states_a_selection_reason` |
| 641 | `validate_resolved_plan` | `malformed-receipt` | `package {…} l1_include_slow must be a boolean` | **nothing** |
| 645 | `validate_resolved_plan` | `malformed-receipt` | `package {…} carries an exclusion exactly when it gates nothing` | **nothing** |
| 656 | `validate_resolved_plan` | `malformed-receipt` | `resolved plan environments must be a list of named records` | **nothing** |
| 666 | `validate_resolved_plan` | `unknown-package` | `{…} is reported as an unchanged reverse dependency and must not also hold a package record` | **nothing** |
| 677 | `validate_resolved_plan` | `unknown-package` | `{…} is compiled as a dependent of {…} and must not also hold a package record` | `test_affected_scope.py:2585` |
| 696 | `validate_resolved_plan` | `unknown-package` | `{…} has no package record; package is the stored identity and every cell must carry one` | `test_every_cell_carries_a_package_identity`, `test_every_problem_is_a_scope_code` |
| 701 | `validate_resolved_plan` | `malformed-receipt` | `{…} claims area {…} but its package record says {…}; area is derived from package, never independently assigned` | `test_area_must_be_derived_from_the_package_record` |
| 706 | `validate_resolved_plan` | `malformed-receipt` | `{…} has no selection reason` | `test_every_area_package_and_cell_states_a_selection_reason` |
| 711 | `validate_resolved_plan` | `malformed-receipt` | `{…} carries dependents but only a check cell compiles them` | **nothing** |
| 723 | `validate_resolved_plan` | `malformed-receipt` | `resolved plan job_estimate must be a non-negative integer` | **nothing** |
| 737 | `_build_records` | `malformed-receipt` | `resolved plan builds must be a list` | `test_builds_must_be_a_list` |
| 756 | `_build_records` | `malformed-receipt` | `{…} key is not a 16-digit hex build key; the key is computed only through ci-build key` | `test_the_key_must_be_a_sixteen_digit_hex_digest` |
| 761 | `_build_records` | `malformed-receipt` | `build key {…} has two owners, {…} and {…}; one key is compiled exactly once` | `test_two_owners_for_one_build_key_are_refused` |
| 770 | `_build_records` | `unknown-package` | `{…} names a package the plan does not select` | `test_a_build_for_an_unselected_package_is_refused`, +3 |
| 781 | `_build_records` | `malformed-receipt` | `{…} compatible_environments must be sorted and free of duplicates` | **nothing** |
| 786 | `_build_records` | `malformed-receipt` | `{…} producer is not among its own compatible_environments` | `test_a_producer_outside_its_own_compatibility_list_is_refused`, +1 |
| 791 | `_build_records` | `unknown-environment` | `{…} names producer {…}, which is not in the plan's environment table` | `test_a_producer_outside_the_plans_environment_table_is_refused` |
| 799 | `_build_records` | `malformed-receipt` | `{…} artifact is {…}, expected {…}` | `test_an_artifact_that_is_not_package_keyed_is_refused`, `test_an_artifact_name_collision_is_refused` |
| 804 | `_build_records` | `malformed-receipt` | `artifact {…} is claimed by two build records` | **nothing — unreachable, see F1** |
| 812 | `_build_records` | `malformed-receipt` | `{…} has no compatibility reason` | **nothing** |
| 826 | `_build_consumers` | `malformed-receipt` | `{…} has no consumer; a build nothing executes is removed rather than scheduled` | `test_an_unconsumed_build_is_refused`, +1 |
| 846 | `_build_consumers` | `malformed-receipt` | `{…} is consumed in {…}, which its producer's contract does not declare compatible` | `test_a_consumer_outside_the_producers_compatibility_is_refused` |
| 853 | `_build_consumers` | `malformed-receipt` | `{…} consumers must be sorted by {environment, gate} and free of duplicates` | `test_unsorted_or_duplicated_consumers_are_refused` |
| 866 | `_build_identity` | `malformed-receipt` | `{…} identity names package {…}` | `test_an_identity_naming_another_package_is_refused` |
| 874 | `_build_identity` | `malformed-receipt` | `{…} identity {…} must be sorted` | `test_identity_lists_must_be_sorted_so_one_configuration_is_one_key` |
| 878 | `_build_identity` | `malformed-receipt` | `{…} identity {…} must be a non-empty string` | **nothing** |
| 884 | `_build_identity` | `malformed-receipt` | `{…} identity rustflags and features must be strings; an absent value is the empty string` | **nothing** |
| 907 | `_build_references` | `malformed-receipt` | `{…} will execute but references no build; a test execution without one would compile its own` | `test_a_test_execution_without_a_build_is_refused` |
| 913 | `_build_references` | `malformed-receipt` | `{…} references build {…}, but only an executing L1, L2, or browser cell consumes one` | `test_a_build_attached_to_lint_or_check_is_refused`, `test_a_reused_cell_may_not_reference_a_build` |
| 920 | `_build_references` | `malformed-receipt` | `{…} references build {…}, which the plan does not carry` | `test_a_dangling_build_reference_is_refused` |
| 926 | `_build_references` | `malformed-receipt` | `{…} references build {…}, which compiles {…}` | `test_a_build_compiling_another_package_than_its_consumer_is_refused` |
| 942 | `_build_references` | `malformed-receipt` | `build {…}/{…} lists consumers {…} but the cells referencing it are {…}` | `test_a_consumer_the_cells_do_not_demand_is_refused`, +3 |
| 951 | `_dependent_seam` | `malformed-receipt` | `{…} must be an object` | **nothing** |
| 959 | `_dependent_seam` | `malformed-receipt` | `{…} names no dependent; a record with none to compile carries no seam at all` | **nothing** |
| 963 | `_dependent_seam` | `malformed-receipt` | `{…} check_args must be a non-empty string` | **nothing** |
| 972 | `_cell_consistency` | `malformed-receipt` | `{…} reusable must be a boolean` | **nothing** |
| 975 | `_cell_consistency` | `malformed-receipt` | `{…} is reused although no evidence may satisfy it` | **nothing** |
| 979 | `_cell_consistency` | `malformed-receipt` | `{…} is reused but its origin is {…}; a reused cell is satisfied by local or prior-local evidence` | `test_a_reused_cell_may_not_claim_ci_origin` |
| 984 | `_cell_consistency` | `missing-receipt` | `{…} is reused but carries no evidence; a reused cell must name the receipt that satisfied it` | `test_a_reused_cell_must_name_its_evidence` |
| 989 | `_cell_consistency` | `malformed-receipt` | `{…} will execute but its origin is {…}` | **nothing** |
| 993 | `_cell_consistency` | `malformed-receipt` | `{…} is an accepted gap with no governing policy entry; an ungoverned gap can never excuse missing coverage` | `test_an_accepted_gap_without_a_policy_entry_is_rejected` |
| 998 | `_cell_consistency` | `malformed-receipt` | `{…} is prohibited but names no constraint` | **nothing** |
| 1002 | `_cell_consistency` | `malformed-receipt` | `{…} is prohibited yet scheduled to execute` | `test_a_prohibited_cell_may_not_be_scheduled` |
| 1006 | `_cell_consistency` | `malformed-receipt` | `{…} is in state 'reused' but its execution is {…}` | **nothing** |

**23 of the 55 rules are pinned by nothing anywhere in the corpus.** That is a
larger gap than the coarse-code problem the spike was called to measure, and it is
not fixable by any rejection vocabulary.

## Step 1 — collision count: 1

The first attempt — matching each asserted substring against the message templates
with placeholders treated as wildcards — is worthless, and worth recording as a
dead end: a placeholder can produce any string, so the shared literal chunk
`"malformed-receipt: "` makes every assertion that quotes the code "producible" by
all 46 sites. It reported 12 collisions, all artefacts of that.

The measurement that stands is empirical. A line-tagged copy of `schema.py` runs
beside the real one; the real one answers the suite so assertions still pass, the
tagged one records which emission line produced each message. A collision is an
asserted substring found in messages carrying more than one distinct line tag.
Over the suite's own fixtures this exercises 39 of 55 sites; a mutation sweep
(delete and retype every field of the plan, of the first area / package / cell /
build / consumer / identity record, plus 40 hand-written cross-field shapes)
raises it to **54 of 55** — the 55th is unreachable, finding F1.

| result | assertion | sites |
|---|---|---|
| **collision** | `"selection reason"`, in `test_every_area_package_and_cell_states_a_selection_reason` (`test_schema.py:321`) | `schema.py:611`, `:633`, `:706` |
| unique | the other 25 asserted substrings measured | one site each |

**Colliding substring:** `"selection reason"`, asserted identically for all three
mutations of a three-iteration loop. **Colliding rules:** area has no selection
reason (`schema.py:611`), package has no selection reason (`:633`), cell has no
selection reason (`:706`).

### The review's "confirmed" collision is not a collision

`review.md` §5.1 and the spike file both state that `test_schema.py:594` and `:613`
"assert the same substring for two different rules". Measured, both reach **one**
rule, `schema.py:913`, whose condition is the single disjunction
`not (execution == "execute" and gate in BUILD_GATES)`. It is one message covering
two causes, not two rules sharing a code. No rejection-code subdivision could
separate them; only the cell label already inside the message does.

## Step 2 — consumer map

| Consumer | Uses | Branches on the code? |
|---|---|---|
| `scripts/ci-plan.rs:337-340` | `evidence_rejections` | no — renders as an `UnorderedList` |
| `just/ci-local.just:311-314` | `evidence_rejections` | no — counts and renders |
| `scripts/ci-plan-tests.rs:54` | fixture `"gate-inputs-changed: …"` | fixture string only |
| `scripts/ci-build-archive-tests.rs:285` | `contract["vocabulary"]["build_rejections"]` | yes — asserts that frozen set, byte-for-byte |
| `scripts/ci-rollup-tests.rs:3553,3565` | `vocabulary.origins`, `accepted_gap_state` | no rejection code |
| `.github/ci/schemas/contract.json:249` | publishes `REJECTIONS` | published, read by nothing |
| `.github/workflows/*.yml` | — | nothing reads a rejection code |
| `.githooks/tests/test-pre-push.sh:741` | `validate_resolved_plan` | emptiness only |
| `affected_scope.py:2843`, `local_evidence.py:497,747,952` | `problems[0]` | no — embeds in a raised message |
| **`scripts/ci/local_evidence.py:257-261`** | assigns plan problems a code | **yes — hard-codes `malformed-receipt`** |
| **`scripts/ci/test_resolved_plan.py:543`** | exact literal message | **yes — `assertNotIn` on the coded string** |
| `scripts/ci/schema.py:1128` | `str.replace("malformed-receipt", "scope-malformed", 1)` | yes — rewrites the literal |
| `scripts/ci/test_evidence_reuse.py:537,741` | `startswith("malformed-receipt:")` | yes, but on note parsing, not plan structure |

No Rust code, workflow, or justfile reads `vocabulary.rejections` at all; only
`build_rejections` is mirrored and asserted. The cross-language risk the spike
feared is therefore not where the gate trips. It trips inside Python:

- `local_evidence.py:261` wraps a plan problem as
  `f"malformed-receipt: the resolved plan is invalid: {problems[0]}"`. That is the
  string that lands in the plan's `evidence_rejections` and reaches
  `ci-plan.rs:337` and `just ci-local --plan`. Re-coding inside `schema.py` alone
  would produce `malformed-receipt: … invalid: plan-…: …` — two codes in one
  rejection — and would change nothing a consumer classifies on.
- `test_resolved_plan.py:543` asserts the full coded literal with `assertNotIn`.
  Change the code or the prose and the assertion becomes vacuously true instead of
  failing.

Both files are outside this spike's ownership, so the gate's verdict applies:
subdividing is a contract change needing its own fix cycle, not a test-quality
improvement.

## What shipped

Only `scripts/ci/test_schema.py`, and only the two disambiguations the plan lists
as shipping regardless of the ruling.

1. `test_every_area_package_and_cell_states_a_selection_reason` — the one measured
   collision. Each of the three mutations now asserts its own exact message
   instead of the shared tail.
2. `test_a_build_attached_to_lint_or_check_is_refused` and
   `test_a_reused_cell_may_not_reference_a_build` — each now asserts the exact
   message, whose cell label is the only thing that separates the two causes of
   `schema.py:913`. Re-measured: 0 collisions, and the three strings are mutually
   exclusive.

Nothing else changed. `schema.py` is untouched by this spike; `REJECTIONS` and
`SCOPE_REJECTIONS` are byte-identical to `HEAD` (verified by importing both
revisions and comparing the tuples); `contract.json` is byte-identical before and
after `python3 scripts/ci/schema.py`, so regeneration is a confirmed no-op.

## Findings

- **F1 [high] `scripts/ci/schema.py:802-806` is unreachable, and its test tests
  something else.** `expected_artifact` is derived from
  `{package, producer, key}`, so two records can only collide on it by sharing a
  key — and a shared key is caught and `continue`d at `schema.py:759-765` before
  the artifact check runs. The `elif` can never be true. It is the one site the
  mutation sweep could not reach. `test_an_artifact_name_collision_is_refused`
  does not reach it either: its fixture trips the
  artifact-*mismatch* rule at `schema.py:799`, which satisfies the `"artifact is"`
  assertion. `test_an_artifact_name_collision_is_refused` is at
  `scripts/ci/test_schema.py:521`. So the collision rule has no test, and the test
  has a name that says it does. Either the rule is dead code to delete or the artifact rule needs to
  admit a hand-spelled artifact for the collision check to have meaning.
- **F2 [high] `scripts/ci/local_evidence.py:257-261`** assigns plan-structure
  problems their published code. Any future granularity work has to start here,
  not in `schema.py`.
- **F3 [medium] 23 of 55 plan-validation rules are pinned by no assertion.**
  Including the whole unpinned half of `_cell_consistency` (`schema.py:972`, `:975`,
  `:989`, `:998`, `:1006`) and all of `_dependent_seam`'s own rules (`:951`, `:959`,
  `:963`). Worth its own item in §1 completeness gaps rather than §5 brittleness.
- **F4 [medium] `scripts/ci/test_resolved_plan.py:543`** keys on the exact coded
  literal with `assertNotIn`; it degrades to vacuous rather than failing on any
  prose or code change.
- **F5 [medium] `review.md:609-611`** states `test_schema.py:594`/`:613` pin "two
  different rules". Measured, it is one rule, `schema.py:913`, with a disjunctive
  condition. §5.1's recommendation rests partly on that misreading.
- **F6 [low] `scripts/ci/schema.py:1128`** rewrites a rejection code with
  `str.replace` because `_sha` and `_str_list` hard-code `malformed-receipt` while
  `_keys` and `_member` take a `code` parameter. That asymmetry is the first thing
  to fix if a fourth vocabulary is ever added. The rewrite is correct today only
  because the prefix is always the first occurrence in the string; it is written as
  a general substring replacement, not as a prefix one.
- **F7 [low] `scripts/ci/test_affected_scope.py:2482`** asserts
  `any("native" in error …)` — one word, and a field name at that, so it matches
  any message mentioning `native`. The weakest assertion found against
  plan-validation output.
- **F8 [low] `review.md:613-620`'s counts are off.** "53 problem literals, 45
  `malformed-receipt`" counts only `problems.append(...)`, which includes
  receipt-side rules and misses every helper-returned or comprehension-built
  message. The plan-reachable surface is 55 sites, 46 of them always
  `malformed-receipt`. The conclusion survives; the numbers should be corrected if
  the review is published.

## Verification

Baseline, established before any edit, by running each suite as a standalone
script from the worktree root — **607 tests, all passing**, across 13 files. The
review's 607 is current; the "590" figure is not.

| suite | tests | before | after |
|---|---:|---|---|
| `test_affected_scope.py` | 174 | OK | OK |
| `test_build_baseline_revision.py` | 11 | OK | OK |
| `test_build_key.py` | 12 | OK | OK |
| `test_ci_local.py` | 62 | OK | OK |
| `test_constraints.py` | 39 | OK | OK |
| `test_cross_check.py` | 13 | OK | OK |
| `test_evidence_reuse.py` | 66 | OK | OK |
| `test_local_evidence.py` | 20 | OK | OK |
| `test_publish_gaps.py` | 19 | OK | OK |
| `test_resolved_plan.py` | 64 | OK | OK |
| `test_reuse_validation.py` | 15 | OK | OK |
| `test_runner_loss.py` | 43 | OK | OK |
| `test_schema.py` | 69 | OK | OK |
| **total** | **607** | **607 pass** | **607 pass** |

No previously-passing test fails. The spike's highest-value possible outcome — a
collision masking the wrong rule firing — did **not** occur: every rule the three
rewritten assertions now pin exactly is the rule that was already firing.

`test_shipped_contract_matches_this_module` and
`test_shipped_contract_is_byte_stable` (`test_schema.py:240`, `:249`) pass, and
`.github/ci/schemas/contract.json` has the same SHA-256 before and after
regeneration.

Not measured, and why: the Rust-side assertion
`scripts/ci-build-archive-tests.rs:271-291` was read, not run. It reads
`contract["vocabulary"]["build_rejections"]`, which this spike did not touch and
which is byte-identical, so running it would prove nothing about the spike; it
belongs to the in-flight single-os work that already owns that file.

## Follow-ups this spike declines to do

1. Decide F1: delete `schema.py:802-806` or make it reachable, and rename or
   retarget `test_an_artifact_name_collision_is_refused`. Needs a `schema.py`
   behavior decision, not a test change.
2. If plan-structure granularity is ever wanted, it is a contract change spanning
   `schema.py`, `local_evidence.py:257-261`, `test_resolved_plan.py:543`, and the
   `code` parameters missing from `_sha` and `_str_list` — its own fix cycle, with
   `contract.json` regenerated to gain the new list.
3. Cover the 23 unpinned rules (F3). Independent of any vocabulary question.
