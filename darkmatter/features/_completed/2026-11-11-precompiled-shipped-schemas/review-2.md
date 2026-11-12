---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-11-11T19:28:52-08:00
spec: 2026-11-11-precompiled-shipped-schemas/spec.md
implemented: false
description: "A **feature** review of `2026-11-11-precompiled-shipped-schemas/spec.md`"
feature: 2026-11-11-precompiled-shipped-schemas/review-2.md
previous: 2026-11-11-precompiled-shipped-schemas/review-1.md
---

# Review 2: precompiled shipped schemas

The feature is **production ready**. Both previous findings are verified fixed,
including every child instance in the repair ledger. There are no new findings,
partial repairs, newly found siblings, or blocked findings.

I read the specification, plan, first review, original implementation log,
and repair log before reviewing the implementation. The review covers the
implementation through `08fac9d87` and the current uncommitted repairs.
The previous review contains no human rulings. Its accepted technical choices
remain accepted.

## Previous Finding Verification

The required instances below are the child rows in the repair log's
`### Finding Ledger`. “Verified fixed” includes previously clean sites whose
expected behavior was rechecked; it does not imply those sites needed changes.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: Downstream freshness scheduling omits stored-digest preparation code | R1-F1.1–R1-F1.7 | verified fixed | Both downstream manifests and freshness functions name the digest-producing code; resolved-plan regression tests pass. Each child is checked below. | Nothing |
| R1-F1.1: Darkmatter scheduling | A change to `simplified-schema/lib/src/validate.rs` selects Darkmatter's freshness test | verified fixed | Exact `source-inputs` declaration and test-local `include_str!`; passing planner regression asserts a narrowed Ubuntu L1 cell containing `test(=shipped_artifacts::shipped_artifact_is_fresh)`. | Nothing |
| R1-F1.2: Claudine scheduling | Same change selects Claudine's freshness test | verified fixed | Same declaration/read pairing; passing planner regression asserts its narrowed Ubuntu L1 freshness filter. | Nothing |
| R1-F1.3: Simplified Schema owner suite | Same change continues to select the owner's full suite | verified fixed | Separate resolved-plan reproduction checks unfiltered Simplified Schema L1 cells for the source change. | Nothing |
| R1-F1.4: Shared wire code | `simplified-schema/lib/src/shipped/wire.rs` selects both downstream freshness tests | verified fixed | Separate resolved-plan reproduction asserts one Ubuntu L1 cell per downstream owner with its freshness filter. | Nothing |
| R1-F1.5: Envelope YAML | `simplified-schema/schemas/schema.yaml` selects both downstream tests | verified fixed | Passing planner regression checks both narrowed freshness filters; a separate resolved-plan reproduction confirms unfiltered owner L1 cells. | Nothing |
| R1-F1.6: Lower artifact | Simplified Schema artifact-only edit selects both downstream tests | verified fixed | Passing planner regression checks both narrowed freshness filters and the owner's full suite. | Nothing |
| R1-F1.7: Middle artifact | Darkmatter artifact-only edit selects Claudine's freshness test | verified fixed | Passing planner regression checks Claudine's narrowed freshness filter and Darkmatter's full suite. | Nothing |
| R1-F2: Root regeneration replaces artifacts before full validation | R1-F2.1–R1-F2.7 | verified fixed | Every generator now accepts emitted bytes before printing them; the coordinator collects all accepted candidates before replacement. Each child is checked below. | Nothing |
| R1-F2.1: First owner refuses | Simplified Schema refusal preserves all three files | verified fixed | Generator L1 refusal test asserts exit 1 and empty stdout; coordinator contract tests exercise this owner and assert unchanged originals and no temporary files. | Nothing |
| R1-F2.2: Middle owner refuses | Darkmatter refusal preserves all three files | verified fixed | Emit mode calls `Registry::load_over(...).check_all()` before output. L1 tests reject bad payloads, wrong owners, dependency mismatches, and mismatched or unreadable lower candidates. Coordinator refusal and lower-mismatch cases preserve every original. | Nothing |
| R1-F2.3: Last owner refuses | Claudine refusal preserves all three files | verified fixed | Emit mode checks every section over both fresh lower fingerprints. L1 tests reject bad payloads, wrong owners, and each lower mismatch; coordinator cases preserve every original. | Nothing |
| R1-F2.4: Successful root publication | All owners accept; all three publish; repeat is unchanged | verified fixed | Coordinator success/repeat contract passes. Real root `just shipped-schemas` exits 0 with `every shipped artifact is current`; tracked artifact bytes remain unchanged. | Nothing |
| R1-F2.5: Simplified Schema area write | Refused fresh bytes preserve the existing artifact | verified fixed | `write` checks acceptance before writing; passing `a_failed_generation_leaves_the_artifact_in_place` checks unchanged bytes and no temporary file. | Nothing |
| R1-F2.6: Darkmatter area write | Same preservation with current lower artifact | verified fixed | Same ordering invariant and passing area-write preservation test. The stale-lower check still precedes write mode. | Nothing |
| R1-F2.7: Claudine area write | Same preservation with current lower artifacts | verified fixed | Same ordering invariant and passing area-write preservation test. Both compiled lower artifacts must still match fresh generation. | Nothing |

The dependency sweep also adds preparation inputs beyond the original missing
file: hashing, scalar conversion, recursive resolution, recorded kinds and
origins, expression declarations, and trigger grammar. I inspected the paired
manifest declarations and test-local reads. The planner regression additionally
checks `claims.rs`, biscuit-hash's `xx.rs`, and trigger `grammar.rs`.
This completes the existing finding's class sweep; it is not a new sibling.

## Unblocked Findings

None.

## Blocked Findings

None.

## Spec coverage and verification level

All requirements here need **Level 1** verification. They concern prepared
schema data, validation decisions, positions, cache identities, editor protocol
results, and headless generation commands. No requirement changes terminal
rendering, keyboard encoding, or browser interaction, so Level 2 or Level 3
verification is unnecessary.

| Requirement | Evidence reviewed or rerun |
| --- | --- |
| Freshness and ordered prepared-data equality | Reran all three owners' artifact suites. Freshness, deterministic generation, ordered round trips, and isolated source mutation pass. |
| Accepted artifacts avoid preparation; stale or custom input falls back | Artifact suites exercise source/dependency/revision failures and malformed records. Claudine's counter-enabled suite confirms accepted lookups prepare nothing and generation consults no artifact. Darkmatter's prepared-bundle suite checks skipped generation-time checks and ordinary preparation for custom inputs. |
| Disk shadowing and imports in surviving bundles | Reran all 15 `prepared_trigger_bundles` tests, including disk suppression, invalid/non-trigger suppression, mixed disk/prepared siblings, and imports that remain bound to bundle text. |
| Deferred positions retain authored coordinates | Reran all four Darkmatter `deferred_positions` tests. Inspected the retained envelope-position coverage and original coordinate fixtures. DMLS's rerun editor suite includes Unicode/CRLF ranges. |
| Validator keys preserve roles, order, imports, templates, phases, and file contexts | Reran all 13 `layered_validator_identities` tests, including canonical fallback, context isolation, structural reuse, and benchmark reset behavior. Inspected Simplified Schema's fixed-width encoding and cross-process identity tests; these are unchanged by the repair. |
| DMLS retains workspace and buffer behavior | Reran all 34 `yaml_document_editor` tests, including shipped kinds, checkout replacement, unsaved imported types, completion, hover, and diagnostic ranges. The existing workspace/buffer tests remain in the declared L1 target. |
| Root regeneration validates before publication | Five real-script coordinator contract tests plus each generator's in-process acceptance/refusal tests pass. Root generation uses fresh lower candidates and succeeds with current artifacts. |
| CI schedules downstream freshness for preparation changes | Two planner regression tests pass, plus the separate shared-wire resolved-plan reproduction. No new CI job or full downstream suite was introduced. |
| No runtime I/O or host-specific artifact state | Registry and descriptor paths remain compiled-data consumers. New candidate-file reads are confined to developer generator examples. Portable identity records, source guards, and source-only fallback remain intact. |
| Fixture cleanup and documentation | Original implementation log records which speed-only reduced-schema fixtures were removed and which custom/shadowing fixtures remain. Topic documentation and skills describe generation, fallback, lazy positions, and remaining runtime validator compilation. |

## Input robustness

The artifact reader's default invalid-input outcome remains silent refusal and
ordinary preparation from compiled text. Header/layout failures refuse the
artifact; under the previously accepted deferred-loading design, a malformed
payload section refuses that section. Nullable metadata preserves explicit null
while required-presence deserializers reject omission where required.

The shared loader's L1 matrix covers missing/null/wrong-type/empty values,
collection element errors, duplicate members, trailing content, invalid JSON,
and JSON5 syntax that this JSON format does not accept. Owner suites exercise
record validation and public fallback results. These tests ran again in this
review. The repair adds no new configuration deserializer: lower candidates
are compared as exact bytes and then used as dependency fingerprints, while
the emitted artifact passes the existing full reader.

I also searched the shared reader and owner registries for `serde(default)`,
`filter_map`, `unwrap_or_default`, and discarded parse errors. The dependency
filter follows explicit refusal checks. Registry `.ok()` calls select the
specified fallback after a recorded refusal. No new realistic wrong-result
case emerged.

## Commands and results

The following are fresh review runs, separate from the implementer's claims:

- Simplified Schema: `just test shipped_artifact` — 100 targeted tests passed;
  the recipe also ran all 1695 minimal-feature tests successfully.
- Darkmatter: `just test shipped_artifact` — 54 passed;
  `just test prepared_trigger_bundles` — 15 passed;
  `just test layered_validator_identities` — 13 passed;
  `just test deferred_positions` — 4 passed;
  `just test yaml_document_editor` — 34 passed.
- Claudine: `just test-library shipped_artifact` — 27 passed;
  `just test-library --features work-counters shipped_artifact` — 29 passed.
- `cargo nextest run -p test-toolkit --test shipped_schemas_coordinator_contracts`
  — 5 passed. This package has no area `test` recipe.
- `python3 -m unittest` for
  `RealWorkspaceTestInputTests.test_shipped_schema_inputs_select_each_downstream_freshness_test`
  and `test_the_shipped_schema_coordinator_selects_its_contracts` — both passed
  in 56.589 seconds. These are Python planner tests, not L1 Rust tests.
- Separate `RealWorkspaceTestInputTests.plan` call for
  `simplified-schema/lib/src/shipped/wire.rs` verifies both downstream filters.
- Root `just shipped-schemas` — exit 0, every artifact current.
- `git diff --check` — clean before review metadata edits.

No Rust test in these runs reached five seconds. Concurrent area recipes can
report another run's shared slow-test summary; I checked the individual nextest
results as well. No terminal or browser window was opened. Existing macOS
linker warnings about the unwind section did not cause failures.

The repair's recorded full area suites and lint runs were inspected rather
than repeated wholesale. The original timing evidence remains an acknowledged
limitation: the Claudine debug timing goal was missed on a loaded host; the
spec calls these local goals and uses deterministic counters to establish work
removal. This review does not turn that accepted limitation into a new gate.
Cross-OS evidence remains with ordinary CI.

No verified false behavioral statement in either implementation record required
an in-place correction. The repair log accurately describes validation before
publication and removal of post-publication checks. The latter is appropriate:
pre-publication acceptance supplies the required guarantee, and freshness tests
remain the byte-equality check.

## Recurrence

None. Neither previous defect remains or recurs in a newly found sibling.
No author decision or follow-up human review is needed.
