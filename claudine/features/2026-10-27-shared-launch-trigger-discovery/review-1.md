---
$schema: feature-review.yaml
ready: false
findings:
  - "R1-F1 (high): File-edit and recovery tests rebuild the context they must retain"
human_review: true
human_review_items:
  - "For later confirmation: keep optional count typing in the document's own schema in the rematching tests; this exercises removal without changing existing cleanup rules."
  - "For later confirmation: keep the missing-file control's existing SchemaValidation error at /input; changing the error classification is outside this refactor."
  - "For information: the reported standalone-cleanup change for non-object instances is unreachable because the instance builder always returns an object; no behavior change needs approval."
  - "For later confirmation: retain the disclosed comparison against the specification's historical allocation baseline; a fresh baseline measurement was not required."
  - "For later confirmation: retain Claudine-only completion checks; public signatures and Darkmatter code did not change, and additional downstream checks were not required."
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-10-27T17:17:13-07:00
spec: 2026-10-27-shared-launch-trigger-discovery/spec.md
implemented: true
description: "A **feature** review of `2026-10-27-shared-launch-trigger-discovery/spec.md`"
feature: 2026-10-27-shared-launch-trigger-discovery/review-1.md
log: /Volumes/coding/wt/rusty-biscuit/feat-work-reduction/claudine/features/2026-10-27-shared-launch-trigger-discovery/log.md
implemented_by: claude/opus
next: 2026-10-27-shared-launch-trigger-discovery/review-2.md
---

# Review of shared launch trigger discovery

The feature is **not ready** because one required regression scenario lacks the specified retained-context test coverage. The ownership refactor follows the design: one discovered owner remains local to pre-validation, both passes prepare independently, and caller records are replaced after cleanup. The review found no incorrect production verdict in the exercised cases.

## Findings

### R1-F1 (high): File-edit and recovery tests rebuild the context they must retain

**Authority: spec, Acceptance 2**, requires testing a trigger "created, changed, removed, or repaired between calls sharing a context." The ownership section also requires later calls to discover afresh, including after an error. The implementation plan explicitly calls for the same source and context throughout the freshness and failed-scan recovery cases.

**Defect class:** a regression test reconstructs the state whose reuse it is supposed to verify, leaving stale results on repeated checks unprotected.

In the claudine package, the [freshness test](/Volumes/coding/wt/rusty-biscuit/feat-work-reduction/claudine/lib/tests/l1/invalid_trigger_findings.rs:370) calls [Fixture::verdict](/Volumes/coding/wt/rusty-biscuit/feat-work-reduction/claudine/lib/tests/l1/invalid_trigger_findings.rs:70) after each file edit. That helper builds a new resolution context and resolves a new source on every call. The [scan-recovery test](/Volumes/coding/wt/rusty-biscuit/feat-work-reduction/claudine/lib/tests/l1/invalid_trigger_findings.rs:403) does the same for its failing and recovered verdicts; its local `context` is used only by standalone cleanup. Thus the tests demonstrate that newly constructed callers see edits, while the acceptance criterion concerns callers retaining their existing state. A user editing a schema and rechecking within the same invocation is the realistic scenario this coverage protects.

Reproduction: `just test-library --features work-counters invalid_trigger_findings::` passes all 14 tests. Following each edited-fixture call through `Fixture::verdict` shows the repeated context construction at line 75 and source construction at line 76. This is a test-coverage finding, not evidence of a wrong production result.

The sweep covered every repeated-check/file-edit scenario in the changed test module:

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F1.1 | `separate_calls_see_trigger_creation_replacement_removal_and_repair` | Absent → created → changed → removed → invalid → repaired trigger | All assertions pass, but each verdict receives a new source/context. | The same source/context must observe each transition, with the existing verdict and warning assertions. |
| R1-F1.2 | `a_scan_that_cannot_read_a_root_file_fails_once_and_recovers` | Invalid UTF-8 → valid trigger | Failure and recovery pass through separately constructed verdict contexts; standalone cleanup uses another retained local context. | Failed pre-validation, standalone cleanup, and recovered pre-validation use one source/context; retain the one-discovery/zero-preparation and error assertions. |
| R1-F1.3 | `the_first_reason_for_a_trigger_survives_later_changes_to_its_file` | One invalid-trigger reason replaced by another | Clean for its ledger contract: the same `TriggerFindings` retains the first reason and delivers once. Contexts are rebuilt. | Preserve the ledger assertions; it does not establish retained-context file freshness. |
| R1-F1.4 | `a_matching_disk_trigger_is_discovered_once_and_prepared_twice` | Two successful calls without a file edit | Clean: the source/context are retained and each call counts one discovery and two preparations. | Already correct; unchanged files cannot substitute for the edited-file scenario. |
| R1-F1.5 | `repeated_checks_record_each_trigger_once_in_discovery_order` | Repeated pre-validation, standalone cleanup, status, and a new ledger | Clean for its delivery contract: filename order and once-only delivery hold; a new ledger reports again. | Already correct for warning delivery; it makes no file-edit assertion. |

**Smallest repair:** construct the source and resolution context once before the first verdict in the freshness and scan-recovery tests, then call public `pre_validate_schema` directly with those references throughout. Keep the existing edits, findings ledger, counters, and observable assertions. This needs no new production code, cache layer, process matrix, or human ruling. Correct the implementation log's claim of same-source/context freshness coverage alongside the test repair.

## Implementation decisions (informational)

Two agents independently read the implementation log and specification before this review compared code against the requirements. None of the following choices requires a human decision before repair:

- **Declare the removable optional value in the document's own schema.** The implementer found that existing cleanup treats a property lacking authored-schema typing as required. Declaring optional `count` in the authored schema and requiring `ticket` in the trigger preserves the intended rematching experiment. Recommendation: keep this choice; expanding trigger-only cleanup would change behavior beyond this refactor.
- **Assert the existing error for a trigger-declared missing file.** The missing-file caller-origin control expects `SchemaValidation` at `/input`, rather than the plan's `UnresolvedFileReference`. The existing classifier depends on authored-schema typing for the latter. Recommendation: keep the actual error assertion; the valid launch-relative file control still exercises surviving caller origins.
- **The reported non-object cleanup departure has no reachable effect.** Phase 1 says standalone cleanup now discovers before recognizing a non-object instance. In the claudine package, [build_effective_instance](/Volumes/coding/wt/rusty-biscuit/feat-work-reduction/claudine/lib/src/composition/schema/mod.rs:1014) always returns `Value::Object`, so the old early return could not be reached either. No public finding behavior changes on that basis, and no restoration is needed.
- **Use the historical allocation baseline.** The log identifies the baseline as historical, records the measurement revision and uncommitted state, profile, toolchain, and six raw rows. This follows Acceptance 3. Recommendation: keep the comparison; a fresh baseline is unnecessary.
- **Keep completion checks within Claudine.** Darkmatter code and public signatures are unchanged. The required Claudine area tests and lint are recorded as passing. Recommendation: accept this scope; extra downstream or Darkmatter checks are not required here.

No required implementation work was explicitly deferred. Broader caching, changes to trigger-only optional cleanup, additional benchmark workloads or hosts, and conditional Darkmatter checks were deliberately excluded by the feature's scope. Cross-OS runtime evidence remains a CI responsibility and does not affect this review's readiness decision.

## Verification and acceptance coverage

The reviewer ran these commands from `claudine/`, through Nextest:

| Command | Result | What it establishes |
| --- | --- | --- |
| `just test-library --features work-counters invalid_trigger_findings::` | 14 passed | Discovery/preparation counts, cleanup rematching, override attribution, typed authored-schema failures, findings retention, and the freshness/recovery assertions subject to R1-F1. |
| `just test-library --features work-counters composition::schema::` | 126 passed | Existing cleanup exclusions, templated and literal values, mode-dependent validation, document-relative schema/file resolution, caller files, and schema status behavior. |
| `just test-cli --features test-fixtures launch_trigger_schemas::` | 57 passed | CLI launch blocking and success, malformed-trigger warning delivery, disk shadowing of embedded triggers, retry/proxy/sequence wiring, and subsequent-run schema edits. |

All 197 selected tests passed; none exceeded five seconds. The linker emitted its compact-unwind size warning during compilation. These are Level 1 in-process and hermetic subprocess checks, which are appropriate for this feature's filesystem, validation, and launch-wiring contracts. No requirement depends on terminal rendering or physical keyboard input.

The changed tests are included by the declared `l1` target despite `autotests = false`; their names remain in L1. Both the local `just test` recipe and CI metadata enable `work-counters`. The main successful counter test constructs its source/context before observation and confirms one discovery and two preparations on each call.

Code inspection also confirmed document-relative discovery before launch-context configuration, embedded bundle attachment, unchanged matching identity and authored-schema precedence, no owner/registry clone, findings recorded before authored resolution, immediate propagation of failed discovery, best-effort cleanup preparation, and independent standalone/status loading. Authored-only loading remains unchanged and trigger-free.

The implementation log records final `just test` results of 11,767 passing tests and clean `just lint`; these full gates were not repeated during review. It also records allocated-byte reductions of 48.3%, 49.7%, and 50.0% at the three required sizes, with discovery/preparation counts changing from 2/2 to 1/2. The review inspected those measurements and benchmark changes without rerunning the allocation experiment.

## Input robustness

This change separates discovery and preparation in the existing schema loader; it introduces no field validator, deserializer, file format, or field default. The existing Darkmatter envelope/payload loaders still decide field and document shapes. Their outcomes must remain intact: invalid individual triggers produce located findings while valid rules continue, infrastructure failures return the translated schema error, and authored-schema failures remain typed errors after findings are recorded.

The review traced those three outcome paths and inspected the changed adapter for permissive parsing patterns (`serde(default)`, null-conflating field options, filtered string collections, `unwrap_or_default`, and discarded parse errors). No new field coercion was introduced. Cleanup's intentionally swallowed preparation error preserves its existing best-effort contract; discovery errors escape pre-validation immediately. Public and CLI tests exercise malformed triggers, malformed authored schema, missing authored files, and unreadable UTF-8. This review does not claim a newly executed exhaustive matrix for the unchanged Darkmatter parsers.
