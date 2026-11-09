---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-11-09T01:28:45-08:00
spec: 2026-11-05-expression-recovery/spec.md
implemented: false
description: "A **feature** review of `2026-11-05-expression-recovery/spec.md`"
feature: 2026-11-05-expression-recovery/review-3.md
previous: 2026-11-05-expression-recovery/review-2.md
---

# Expression recovery review, iteration 3

The feature is **production ready** under the specification's typed read-failure
boundary. No new findings remain. The recursive environment-reference defect
from review #2 is fully repaired; the three findings from review #1 remain
fixed. No human decision is needed to complete this review.

## Previous Finding Verification

Read both earlier reviews and the repair's `log.md`, including both
`### Finding Ledger` sections, before assessing the implementation. Neither
earlier review contains a `## Human rulings` section. The spec's decisions
about typed read failures and unsupported formats remain in force. Previously
accepted implementation choices are not reopened.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R2-F1: Recursive missing-source format checks use unexpanded environment references | R2-F1.1–R2-F1.16 and R2-F1.S1–R2-F1.S6; checked individually below | **verified fixed** | Shared interpolation implementation, public composition tests, six biscuit-file API tests, and independent CLI missing/present pairs | Nothing; this repairs the sibling/regression identified in review #2, rather than leaving a partial repair. |
| R1-F1: Missing sources bypass invalid-request checks | R1-F1.1–R1-F1.23, already verified fixed in review #2 | **verified fixed** | Reran computed missing/present request tests, `load_data`, call checks, candidate-planner tests, and recovery tests; inspected shared validation ordering | Nothing; R2-F1's additional environment-reference instance is now fixed too. |
| R1-F2: Stale remote cache hides a denied redirect | R1-F2.1–R1-F2.19, already verified fixed in review #2 | **verified fixed** | Reran remote and recovery tests, including warm-cache refusal and transport controls; inspected the shared cache branch and typed adapters | Nothing; stale responses remain restricted to transport failures. |
| R1-F3: Value-only try calls return a generic handler error | R1-F3.1–R1-F3.5, already verified fixed in review #2 | **verified fixed** | Reran direct value-call and ordinary/lifecycle callback rejection tests; inspected the common contract-error branch and host-binding distinction | Nothing |

### Review #2 instance verification

Paths below are relative to the repository root. The nine independent CLI
pairs used temporary documents, `DATA_FILE=optional.yaml`, `DATA_EXT=yaml`,
and `DATA_DIR=nested`, through `target/debug/md compose --output json`.
Every successful case asserted an empty diagnostic list and parsed document
value; every invalid case asserted failure and its diagnostic code.

| Required instance | Verification evidence and result |
|---|---|
| R2-F1.1 | `%{{DATA_FILE}}`: public `recursive_env_file::{missing,present}` tests and independent CLI pair return `[]` when absent and `["loaded"]` when present. |
| R2-F1.2 | `%optional.{{DATA_EXT}}`: public `recursive_env_extension::{missing,present}` tests and CLI pair give the same correct results. |
| R2-F1.3 | `%{{DATA_FILE}}` with YAML and `delimiter: ";"`: public `recursive_env_file_delimiter::{missing,present}` tests and CLI pair reject both states with `dm.data.invalid_options`; public tests assert no fallback read. |
| R2-F1.4 | `%{{DATA_DIR}}/optional.yaml`: public `recursive_env_dir::{missing,present}` tests and CLI pair preserve missing recovery and present loading. |
| R2-F1.5 | `{{DATA_FILE}}`: public `env_file::{missing,present}` tests and CLI pair preserve nonrecursive behavior. |
| R2-F1.6 | `optional.{{DATA_EXT}}`: public `env_extension::{missing,present}` tests and CLI pair preserve nonrecursive extension handling. |
| R2-F1.7 | `%{{DATA_FILE}}` with `format: "yaml"`: public `recursive_env_file_explicit_format::{missing,present}` tests and CLI pair preserve explicit-format behavior. |
| R2-F1.8 | `%optional.yaml`: public `recursive_supported_extension::{missing,present}` tests and CLI pair recover only the miss. |
| R2-F1.9 | `%optional.xyz`: public `recursive_unsupported_extension::{missing,present}` tests and CLI pair reject both states with `dm.data.unsupported_format`, preserving the author's decision. |
| R2-F1.10 | All nine present CLI controls above select the expected format/applicability result. `load_data_fn` derives a present local source's format from its resolved identity. |
| R2-F1.11 | Reran `call_check` and public composition cases. `provable_format`/`format_of_path` inspect literal text only; an unknown extension does not prove a rejection and performs no environment lookup or source read. |
| R2-F1.12 | `evaluate_recovery` still delegates failure classification to `is_recoverable_read_failure`; invalid requests propagate and only valid misses recover. Classifier and evaluator tests pass. |
| R2-F1.13 | Lifecycle `call_recovery` uses that same typed classifier and loader result; no separate filename/format derivation exists there. Lifecycle recovery tests selected by `try` pass. |
| R2-F1.14 | Inspected `AbsentCandidate::identity`: recursive/unplanned candidates retain authored text separately from the expanded format path. `an_authored_candidate_names_the_reference_as_written` passes. No resolved path is invented for the absent reference. |
| R2-F1.15 | Inspected the rewritten `absent_candidates` documentation: it now describes environment-expanded payloads and separately retained authored identities. The old raw-extension assertion is gone. |
| R2-F1.16 | CLI `recursive_env_reference_recovers_only_a_miss` runs in the declared L1 target and passes; the independent pair also confirms its result. |
| R2-F1.S1 | Passive format inference remains text-only; inspection and call-check tests confirm no new source execution. An unprovable format is left to runtime checks. |
| R2-F1.S2 | Present local reads still pass the resolved identity's extension into shared `plan_read`; the present controls pass. |
| R2-F1.S3 | Remote format selection and fetching still use the same URL identity, outside the new local missing-source branch. Remote tests pass. |
| R2-F1.S4 | Inspected recursive-reference branches in expression filesystem helpers and schema validation. They handle paths, probes, or search roots; none derives data format or option applicability from the raw recursive reference. |
| R2-F1.S5 | Inspected `file_extension` and raw-reference diagnostic uses. Neither supplies the missing-source request verdict; the verdict is owned by `data/load.rs`. |
| R2-F1.S6 | `FileReference::interpolated_payload` delegates to the existing strict local interpolator or lenient URL interpolator through the captured context. All six public API tests pass: unchanged payload, whole filename, extension, directory, unset local variable, and unset URL variable. The method performs no filesystem probe. |

The repair adds one small public helper to **biscuit-file** and uses it in
**darkmatter**'s missing-source candidate derivation. It does not duplicate
environment expansion or broaden the recovery classifier. The corresponding
loading-data and file-reference documentation describe the implemented behavior.

## Unblocked Findings

None.

## Blocked Findings

None.

## Recurrence

No recurring finding remains in this review. R2-F1 was the regression in
R1-F1's request-validation class identified by review #2; its full recorded
instance inventory is now verified fixed. No additional sibling was found.

## Verification and acceptance coverage

All commands ran in this session's worktree. Filtered runs overlap and their
counts must not be added as distinct coverage. These are reviewer-run results,
separate from the implementation log's full-suite and lint claims.

| Command | Result |
|---|---|
| `darkmatter/`: `just test recovery` | 258 passed |
| `darkmatter/`: `just test try` | 568 passed |
| `darkmatter/`: `just test load_data` | 505 passed |
| `darkmatter/`: `just test remote` | 260 passed |
| `darkmatter/`: `just test direct_value_call` | 2 passed |
| `darkmatter/`: `just test callback_function` | 2 passed |
| `darkmatter/`: `just test data::load::` | 8 passed |
| `darkmatter/`: `just test call_check` | 112 passed |
| `biscuit-file/`: `just test interpolated_payload` | 6 matching API tests passed; recipe's additional minimal-feature checks passed |
| Root: `just check-tier-coverage darkmatter` | Zero stranded tests |
| Root: `just check-tier-coverage biscuit-file` | Zero stranded tests |
| Independent debug CLI reference matrix | All nine missing/present pairs passed |

No recorded L1 test reached five seconds. An initial concurrent `try` run
passed its tests but lost its slow-test report to another run using the same
nextest report location. A sequential rerun passed all 568 tests and recorded
the slow-test result successfully. That verification artifact race is not a
product defect. Local linking emitted an unwind-table size warning; compilation
and the tests completed successfully.

| User-facing requirement | Strongest verification inspected/rerun | Assessment |
|---|---|---|
| Optional data through normal composition; missing, unreadable, malformed, and mismatching sources | L1 public composition and CLI tests; guarded real read path for unreadability; independent CLI reference pairs | Appropriate; passes. The accepted library-level unreadability fixture remains sufficient. |
| Invalid format, options, contracts, imports, reference syntax, and boundaries remain fatal | L1 public request matrix, passive checks, classifier tests, CLI failures | Appropriate; passes. |
| Falsy successes, once-only operands, lazy fallback, nested recovery, callback scope, shared iteration allowance | L1 instrumented evaluator and public composition tests | Appropriate; passes. The success branch does not evaluate the fallback or clear the memo. |
| Remote permission refusal, warm caches, allowed transport failure, invalid UTF-8, discovery deferral | L1 mocked transport, public composition, cache, CLI and preflight tests | Appropriate; passes. Policy refusal still reaches callers before recovery. |
| Body/frontmatter, page conditions, transclusion targets/conditions, callbacks, collection regions, shell templates | L1 normal-entry-point surface tests with success, recoverable failure, and invalid-request controls | Appropriate; passes; existing surface restrictions remain intact. |
| Lifecycle recovery and fatal non-read failures | L1 executor tests, including template operands, plus shared handler/classifier inspection | Appropriate; passes. |
| Schema-slot permission checks, determinism, empty candidates, selected reads, dependency discovery | L1 public schema/session tests | Appropriate; passes. |
| Quiet handled failures, retained warnings, blocking warnings, callback provenance, fallback locations | L1 report assertions and CLI JSON-envelope tests | Appropriate; passes; accepted interpolation-level location precision is preserved. |
| Read sets, source memo failures, distinct fallbacks, invalidation after effects | L1 public data-runtime and composition tests | Appropriate; passes; accepted unresolved-probe identity behavior is preserved. |
| Union typing, nullable success, typed empty-array fallback, constrained arrays, unknown contracts | L1 passive type and callback tests, including DMLS completion | Appropriate; passes. |
| Editor completion, hover, arity checks, static reads without execution; host-name collisions | L1 DMLS and registration tests | Appropriate; passes. |
| Catalog and generated documentation agree with recovery behavior | L1 catalog tests plus inspection of generated `try` page and relevant topic pages | Appropriate; passes; the previously accepted catalog fallibility convention is unchanged. |

The recovery modules are declared by the consolidated L1 targets, and both
area tier audits pass. The spec promises expression results and diagnostics,
not terminal-emulator rendering or keyboard interaction, so no Level 2 or
Level 3 evidence is required. Cross-OS evidence is left to CI as requested.

### Input robustness scope

The reviewed changes affect an expression special form, typed error handling,
request-check ordering, and reference interpolation. They add no configuration
loader or deserializer and do not change data decoders. The configuration
robustness matrix therefore does not apply. The missing-source helper's
planning `.ok()` and interpolation fallback were inspected: resolution has
already rejected interpolation/context failures before this branch; the
fallback does not turn such a failure into recoverable missing data.

Only review metadata and this review were changed during this assessment.
No production code was edited, no formatter was run, and no commit was made.
