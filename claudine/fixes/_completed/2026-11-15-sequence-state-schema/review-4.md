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
created: 2026-11-15T23:17:27-08:00
spec: 2026-11-15-sequence-state-schema/spec.md
implemented: false
description: 'A **fix** review of `2026-11-15-sequence-state-schema/spec.md`'
fix: 2026-11-15-sequence-state-schema/review-4.md
previous: 2026-11-15-sequence-state-schema/review-3.md
---

# Sequence state schema review, iteration 4

The fix is **production ready**. Both findings carried by review 3 are
verified fixed, including their repair-ledger children. The earlier schema,
ownership, source-context, shell-output, and admission repairs remain fixed.
No new actionable defect or author decision was identified.

## Previous Finding Verification

Read `review-3.md`, its repair's Finding Ledger in `log.md`, both earlier
reviews, `implementation-log.md`, and `implementation.md` before checking the
implementation. No earlier review contains a Human rulings section, and no
finding was blocked. The research-fleet dry-run trade-off is unchanged and
does not require another decision.

The evidence below combines source inspection with this review's passing
sequence run. Completion flags were not used as proof.

The Claudine library's
[`with_scope` and `record_document_scopes`](../../lib/src/composition/sequence/preflight/providers.rs)
retain the authored reads used to predict a prompt task's provider and model.
[`Loader::enter_step`](../../lib/src/composition/sequence/preflight/mod.rs)
keeps those reads paired with the current step's values. This preserves
runtime-dependent selection while keeping unrelated document properties out
of the prediction.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R2-F1 — Prepared provider dependencies are restored only one level deep | Original R2-F1.1–R2-F1.22; review-3 children R2-F1.23–R2-F1.31; repair-added R2-F1.32–R2-F1.33 | verified fixed | All 58 `provider_dependencies_tests` cases passed. `with_scope` follows the complete authored closure through the current step's `step_authored`; `record_document_scopes` records document and `with:` reads in their respective scopes. `defer_runtime_written` follows those closures. The two-link CLI case exits successfully with two local Goose fixture launches. Original prepared-shell, inline-membership, event, and capture tests passed. Child accounting appears below. | None. This completes the partial repair reported by review 3. |
| R3-F1 — Dependency restoration imports unrelated document properties | R3-F1.1–R3-F1.10; repair-added R3-F1.11 | verified fixed | The walk-wide `prepared_replacements` list and name-based append are absent. Every prediction records dependencies from its supplying scope. Owner-collision, independent-sibling, literal-owner, literal-`with`, and caller-setter controls passed. The CLI prompt-owned `choice` case refuses unavailable Claude before any task starts and records zero launches. The overlay invariant is checked below. | None. This repair closes the sibling introduced by the previous restoration mechanism; the original template-ownership repairs remain closed. |
| R1-F1 — Referring-document properties still enter referenced templates | R1-F1.1–R1-F1.9: four reference routes, discovery, direct/precedence/input controls, fleets/docs | verified fixed | Neither `source.rs::plan_referenced_sequence` nor `preflight/inputs.rs::Discovery::walk_referenced` merges referrer properties. Referenced-template, discovery, missing-input, and research artifact tests passed. Fleet declarations and docs retain the initialization-write channel and dry-run setter. | None. |
| R1-F2 — Relative task references and file inputs lose their authoring folder | R1-F2.1–R1-F2.9: prompt/task/sequence/group references, direct/nested controls, relative/absolute `with:`, approval/barrier/execution | verified fixed | Root-reference rows retain `document_scope.declaring_path`; task origins reach discovery, graph construction, `PromptReference.with_origin`, and shared `task_caller_records`. Relative-reference and `with_file_provenance` CLI cases passed, including competing author/launch folders. | None. |
| R1-F3 — Nested templates read raw document values instead of prepared values | R1-F3.1–R1-F3.7: root/inline/group/task-file/referenced owners, discovery, state validation/consumption | verified fixed | `owner_template_values` reads the prepared owning document. The six-placement/four-value-variant/three-builder matrix, setter/item-shadowing controls, and CLI template-fill cases passed. | None. |
| R1-F4 — Referenced document schemas are not checked against current runtime values | R1-F4.1–R1-F4.13: reference routes, direct/inline/valid controls, empty/loop cases, row/event owners, defaults, initialized inputs; transferred R1-F4.14 is covered by R2-F1 | verified fixed | `prepare_whole_document`, referenced-owner execution preparation, root-reference judgment, and root-initialization input layering retain the intended boundaries. Library `referenced_owner_schema` and CLI valid/invalid-write, empty-owner, loop, event-value, and initialized-input cases passed. | None. The original shell-output sibling is also fixed. |
| R1-F5 — Edited inline checks reject containers after initialization has already run | R1-F5.1–R1-F5.8: four placements, unchanged/false-guard/retry/loop controls, whole-document proxy | verified fixed | `admit_inline_container` rereads the check after the guard and before opening the run. `stale_admission`, edited-check CLI cases, frozen-graph cases, and proxy controls passed. Rejected admission produces no initialization effect. | None. |

Every child of the two most recent findings was checked as follows:

| Required children | Verification evidence and result |
|---|---|
| R2-F1.1–R2-F1.5 | First shell rows retain prepared derived values on direct, root-reference, nested-reference, group-leg, and task-file routes; CLI output cases passed. |
| R2-F1.6–R2-F1.10 | Inline first-child shells retain those values on the same routes; CLI output and approval-membership cases passed. |
| R2-F1.11–R2-F1.12 | Root and referenced-document lifecycle shells read prepared values; corresponding CLI cases passed. |
| R2-F1.13–R2-F1.14 | Inline sequence and group lifecycle shell cases passed under direct and referenced owners. |
| R2-F1.15–R2-F1.16 | Shell-valued writes capture the prepared value on direct and reference routes; CLI cases passed. |
| R2-F1.17–R2-F1.20 | Later-position, literal, rejected side-effect-shell spelling, and template/state controls passed. These consumers do not use provider dependency restoration. |
| R2-F1.21–R2-F1.22 | Prepared-view late/runtime/environment/shell-derived controls passed. The early-binding comments, sequence topic docs, preflight docs, and repo skill match the implementation's prepared-value/retained-expression distinction. |
| R2-F1.23–R2-F1.28 | Two- and three-link `with.agent` and `with.model` chains defer on all six placements in `derived_with_chains_wait_for_the_write`; unwritten-chain controls stay static. The direct two-link CLI reproduction launches the writer and reader successfully. |
| R2-F1.29 | Prompt-owned provider chains of zero through three links and model chains of zero, one, and three links defer on all six placements. Both fields use the same closure implementation. |
| R2-F1.30 | Direct-read and one-link `with:` controls defer for both selection fields on all six placements. |
| R2-F1.31 | Shell and template/state consumers remain independent of provider closure recording; their original tests passed. |
| R2-F1.32 | `a_document_hint_through_a_derived_with_entry_waits` passed on all six placements for provider/model reads, with an unwritten-chain static control. The dependency supplying the `with:` value is followed in sequence scope. |
| R2-F1.33 | `an_identity_derived_with_read_is_circular` passed for `ctx.agent` and `ctx.model` on all six placements. `layer_prepared_values` seeds identity-dependent values as unstable, retaining their expressions transitively for classification. |
| R3-F1.1–R3-F1.7 | Prompt-owned literal `choice` stays static for provider and model at all six owner placements and beside an independent referenced sibling. CLI early refusal records zero launches. |
| R3-F1.8 | Literal-owner controls stay static at all seven placements. |
| R3-F1.9 | Literal `with:` overrides stay static at all seven placements. |
| R3-F1.10 | `caller_setter_overrides_stay_static` passed at all seven placements. `predict_key` returns the caller setter before constructing dependencies. The repair ledger cites a different test here; these are the actual verifying cases. |
| R3-F1.11 | Source invariant: `SequenceStepOverlay::as_set_overrides(None)` contributes only `seq`; `enter_step` removes every overlay key from `seed_authored` before installing `step_authored`. An overwritten value therefore cannot recover its owner's authored reads. The ledger's cited prompt-owner test is not a dedicated overlay test; this invariant establishes the required result. |

## Spec coverage and verification level

Reviewed the key split, declaration validation, authored/composition views,
normalization, document preparation, input discovery, runtime checks,
rereads, frozen task graph, and the dependency repair. Used the `claudine`,
`rust-testing`, and `just` skills.

| User-facing requirement | Verification present |
|---|---|
| Root `$schema` describes YAML/Markdown documents; inputs are collected/coerced directly and through references, including empty lists | Level 1 public preparation/discovery tests and CLI document-input, setter-order, empty-list, and frozen motivating-reproduction cases |
| `$state` validates normalized immediate state passively; templates read prepared owner values with item shadowing | Level 1 resolver/declaration tests, owner-value matrices, state failure-location checks, and CLI filled-state/output cases |
| Declarations remain outside ordinary interpolation; nested owners keep their own contracts | Level 1 composition-view/reread, nesting/ownership, format, placement, and reserved-write tests |
| Inline checks follow guards, precede initialization, and refresh at existing retry/pass boundaries | Level 1 in-process execution plus CLI false-guard, edited-check, retry/loop, and no-initialization-effect assertions |
| Whole referenced documents collect inputs and judge current values using their declaring context | Level 1 public discovery/runtime tests and CLI root/nested/group/task-file, file-provenance, and runtime-write cases |
| Approved children, identities, order, and template-derived state stay frozen | Level 1 frozen-graph, retry/loop, cache/work-count, and command-membership tests |
| Supported whole-document formats agree; data selections do not activate document declarations | Level 1 shared-loader/resolver matrices and existing CLI format wiring cases |
| Shipped and disk schemas agree and the shipped artifact is fresh | Level 1 schema parity and shipped artifact tests |
| Prepared shell values and provider dependencies preserve their supplying values and scopes | Level 1 graph/control matrices plus CLI actual-output, launch-count, and early-refusal assertions |

These are data and execution requirements. None requires terminal-emulator
rendering or OS keyboard encoding, so Level 2 or Level 3 evidence is not
missing.

Verification performed on macOS, with build/test commands run sequentially:

- `just test sequence` in `claudine/`: **2,163 passed**, 10,315 outside the
  filter; 86.746 seconds of execution; none at or above five seconds. This
  includes all 58 new dependency cases and both review-3 CLI reproductions.
- `just test-library shipped_artifacts::`: **27 passed**; none at or above
  five seconds, including artifact freshness and uncached parity.
- `just lint` in `claudine/`: **exit 0**.
- `just check-tier-coverage claudine`: **zero stranded tests**.
- `git diff --check`: **exit 0**.

The library test file is declared in the existing module tree. CLI cases
are declared by `tests/l1/main.rs`; their `test-fixtures` feature is enabled
by the canonical recipe and CI metadata. No production code or tests were
changed during this review. The repair log separately records a full-area
run of 12,463 passing tests with no slow tests; this review reran the affected
sequence and artifact scope. Cross-OS evidence remains CI's responsibility.

## Input robustness matrix

Default outcome: invalid declaration shapes produce typed, located errors
before the rejected task launches. Collection and passive preparation launch
nothing. Existing document defaulting and initialization semantics remain
unchanged.

- **Shared schema-declaration validator:** absence declares nothing; `{}`
  is open. Supported references, unions, and imports resolve. Null, empty
  reference/union, wrong scalar type, and mixed/all-invalid union elements
  fail. Public resolver tests cover empty/nonempty lists; inline check and
  state wiring tests verify owning file/property locations and typed causes.
- **Template validator:** absence supplies no defaults; `{}` is a no-op.
  Null and non-mappings fail. Mapping values intentionally allow literal
  null/array/object data, while string values evaluate. Reserved declaration
  and generated-state destinations fail.
- **State validator:** `the_state_validator_judges_every_field_shape_passively`
  checks a valid control, missing/null required scalars, wrong whole-field
  types, mixed/all-invalid collection elements, allowed empty values, and
  passive conversion without changing the authored plan. Document preparation
  tests separately verify absent/null defaults and input coercion.
- **Document loaders:** the sequence run passed both YAML loaders' duplicate,
  trailing-invalid-content, bare-key-null, and second-document cases; Markdown
  frontmatter controls; the shared JSON/JSON5 loader's duplicate detection,
  comments/trailing commas, and trailing-invalid-content cases; and line-delimited
  blank-line/invalid-record cases. Public resolver and CLI wiring cases confirm
  failures reach callers with locations and no task launch.
- **Smell search:** declarations are read by map presence before being stored
  in `Option` fields, so explicit null is preserved for validation. No new
  load-bearing serde default or silently filtered element parser was found.
  `document.rs`'s empty fallback represents absence of a supplied preparation
  context and does not swallow a parsing error. The provider repair adds no
  file-format loader or field validator.

## Unblocked Findings

None.

## Blocked Findings

None.

## Recurrence

No open recurring finding. R2-F1's incomplete transitive dependency repair
and R3-F1's owner-collision sibling are both closed. The repair now retains
the authored dependency closure with the prediction and its evaluating scope;
it no longer reconstructs ownership from a walk-wide list of names.

## Implementation record corrections

Corrected remaining present-tense referrer-layer and root-reference-directory
claims in `implementation-log.md`, including references to tests that were
replaced by owner-isolation tests. Dated notes preserve the historical phase
context, state current behavior, and cite this review's source checks and
passing tests. The dependency paragraph now clearly describes the pre-repair
iteration-3 state, followed by the verified iteration-4 result.

`implementation.md` already describes these repaired behaviors accurately.
The repair ledger's mismatched test citations are accounted for in the child
verification table above. No finding or human-review item was created for
record maintenance. The spec's design text and review 3's `implemented` and
`implemented_by` claims were left unchanged.
