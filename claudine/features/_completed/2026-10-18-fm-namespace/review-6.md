---
$schema: feature-review.yaml
ready: true
findings: []
observations:
    - The scalar ctx example conflicts with the existing composition contract
    - Non-interactive session documentation still names retired harness validation
    - An unchanged schema test exceeded the Level 1 time budget
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-13T11:26:46-07:00
spec: 2026-10-18-fm-namespace/spec.md
implemented: false
description: A **feature** review of `2026-10-18-fm-namespace/spec.md`
feature: 2026-10-18-fm-namespace/review-6.md
previous: 2026-10-18-fm-namespace/review-5.md
---

# Review of the `fm` namespace, iteration 6

The feature is **production ready** under the specification and the author's recorded exact-text approval ruling. The remaining finding from review 5 is implemented, including tests through document entry and reuse routes. No new high, medium, or low finding was established. The observations below do not require another repair cycle.

## Previous review and repair disposition

Reviewed the specification, reviews 1–5, the [repair log](log.md), the [implementation inventory](implementation-log.md), and the [plan](plan.md). Review 5's `Unblocked Findings` section originally contained none. Its environment finding remained under `Blocked Findings`, but the author's later ruling explicitly unblocked it **before** the latest repair. The repair reports one fixed finding, none deferred, and none disputed.

The author chose to retain approval of exact command text: a later task re-resolves preflight-approved text under its own environment, executes identical text, and refuses changed text before execution. Just-in-time prompt documents retain their own runtime approval. The implementation inventory records this temporary departure from the specification's template-approval sentence. This review uses that ruling; it neither requires the future template-approval implementation nor requests another design decision.

### Verification of the previously outstanding class

The earlier class was environment consumers retaining invocation-wide or live process inputs instead of each document's captured map. The following table accounts for all sibling sites carried by review 5. Executed evidence comes from supplied-map tests; code inspections supplement those tests rather than standing in for route coverage.

| Site and purpose | Shape checked | Result and evidence |
| --- | --- | --- |
| Claudine [invocation and document ownership](../../lib/src/invocation_context.rs) | Launch under A, enter B, then prepare A again | Clean: document views replace environment-dependent state together while retaining launch facts. `invocation_context::document_tests` checks expressions, context values, references, schema roots, approval, selectors, and loop inputs; the first document keeps A. |
| Claudine [bootstrap and re-preparation](../../cli/src/commands/wrap/composition/staged_boot.rs) | Target map captured before initialize; subsequent attempts reuse it | Clean: epochs belong to the entered document view. CLI proxy tests observe the target map during initialize, success, and finalize. |
| Claudine [sequence step entry](../../cli/src/commands/wrap/sequence/iterate.rs) | Successive steps capture distinct maps | Clean: entry captures before composition and passes that view downstream. `successive_sequence_steps_each_read_the_map_they_captured` asserts expression/reference agreement and one capture per step. |
| Claudine [serial group member entry](../../cli/src/commands/wrap/sequence/task_run.rs) | Two serial legs read independent maps | Clean: `open_member_run` captures each leg's view. CLI side-effect, shell, prompt/transclusion, and model-selection tests exercise this route. |
| Claudine [parallel group leg entry](../../lib/src/composition/sequence/task/group.rs) and its CLI runner | Distinct leg maps with shared repository evidence | Clean: `open_leg_run` captures views in declaration order; each worker receives its leg's context and document. CLI tests assert each leg's expression/reference output and setup, primary, and teardown shell environment. |
| Claudine [proxy commit](../../lib/src/composition/coordinator/commit.rs), [adoption](../../cli/src/commands/compose/prep.rs), and sequence proxy loop | Source A selects a target; target lifecycle captures B | Clean: source resolution stays on the source map, then adoption captures the target map before preparation. CLI proxy, failure/retry, and resume tests verify outputs, provider environments, and capture counts. The library proxy snapshot test now uses supplied maps. |
| Claudine [model selectors](../../lib/src/composition/select.rs), [step selection](../../cli/src/commands/wrap/sequence/resolve.rs), and [launch rebuilding](../../cli/src/commands/wrap/harness_orch/loop_control/target_launch.rs) | Launch model differs from step/leg model; repeated attempt retains its map | Clean: environment-derived models read document inputs. CLI tests compare provider argv with `env.MODEL` in composed prompts; explicit CLI and reviewed model choices retain their precedence. |
| Claudine [loop settings](../../lib/src/composition/looping/config.rs) and lazy evidence in the invocation owner | Supplied settings and environment-derived context differ from ambient values | Clean: loop inputs and environment-derived lazy evidence use the document map. Document-consumer tests assert model/agent evidence and all three loop settings. Mutable repository facts remain refreshable. |
| File references, `SCHEMAS_DIR`, transclusion, and [canonical preparation](../../lib/src/composition/prepare.rs) | All consumers read B; included content shares B | Clean: document-consumer tests assert resolved titles, schema-root selection, and approved command text; the CLI prompt-leg test asserts both prompt and included-file output. The sequence snapshot test now supplies a second document map. |
| Claudine [approval recheck](../../lib/src/composition/sequence/preflight/reapproval.rs) and [task execution](../../lib/src/composition/sequence/task/mod.rs) | Same map, unrelated change, and changed command text | Clean: unchanged text runs; changed text fails before task stages execute. Library tests cover setup and primary commands; the shipped CLI test checks all three map cases and proves refused text never runs. Primary, setup, and teardown records share the same verification path. |
| Claudine lifecycle/task runners and Darkmatter [shell execution](../../../darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs) | Child reads a document variable without expression interpolation | Clean: CLI task/group tests observe the captured map in shell children; the model/body test also checks a body directive, lifecycle shell, and shell-valued `set`. Darkmatter tests prove supplied child environments replace inherited values and pin child `PWD` to the working directory. |
| Retired `current_env` readers/classifiers and the namespace migration | Ordinary declared/absent properties; exact reserved-root list | Clean: public composition and editor tests pass; fixed roots remain exactly `fm, ctx, env, current`, and Claudine's late list remains `["current"]`. |

The 13 CLI document-environment route tests are in [document_environment_routes.rs](../../cli/tests/l1/document_environment_routes.rs), declared by the consolidated L1 target and enabled through the area's `test-fixtures` feature. They supply maps to the child process without changing the test process environment. Retry, resume, loop iterations, handlers, and finalize reuse the entered map; capture-count assertions prevent silent recapture. The library document-consumer tests separately verify schema and approval inputs through public preparation results.

## Unblocked Findings

None.

## Blocked Findings

None. The approval ruling is implemented and requires no further human decision.

## Recurrence

The environment finding repeated across reviews 1–5 is closed. All previously listed entry routes and consumer categories are accounted for above. No finding in this review repeats an earlier class, so `recurrence` is `false`.

## Observations

### The scalar ctx example conflicts with the existing composition contract

The specification's `ctx: notes` example conflicts with Darkmatter's existing [context merge](../../../darkmatter/lib/src/markdown/compose/context/merge.rs), which rejects scalar document context without override permission. Namespace tests use a mapping and demonstrate that `fm.ctx` reads document data independently of merged context. This restriction predates the feature; a future example can use a mapping rather than changing the merge contract in this cycle.

### Non-interactive session documentation still names retired harness validation

Claudine's unchanged [non-interactive session topic](../../docs/topics/non-interactive-sessions.md) still lists retired harness validation. The migrated dispatch adapter names its actual template, condition, and matcher consumers. The older sentence remains separate documentation maintenance, as already recorded in earlier reviews.

### An unchanged schema test exceeded the Level 1 time budget

During concurrent full-area runs, Darkmatter reported three slow tests, including [the schema presence matrix test](../../../darkmatter/cli/tests/l1/schema_validate_core_types.rs) `null_admitting_presence_matches_the_phase_a_table_end_to_end`, which crossed five seconds. That test is outside this feature's changes; no feature-specific targeted test exceeded five seconds. This is test-maintenance information, not a new namespace or document-environment repair requirement.

## Requirement coverage and verification

| User-facing requirement | Appropriate level | Evidence |
| --- | --- | --- |
| `fm` snapshots, dotted/bracket reads, literal-key dependencies, missing values, effective overrides, and shell-token selection | Level 1 | Public namespace contract tests and runtime lookup tests pass. Dotted reads traverse the effective map and clone only the selected value; no material performance regression was established. |
| Ordinary `doc`/`current_env`, fixed-root precedence, lifecycle globals, and `set` visibility | Level 1 | Public composition, binding, lifecycle, and sequence-owned-property tests pass in the full suites. Earlier snapshots remain values rather than live views. |
| Hook payload in templates, conditions, matchers, and report messages | Level 1 | Hook consumer tests pass in Claudine's full suite; retained bare/nested payload paths and hook-only scope remain intact. |
| Per-document capture, consistent consumers, shell approval, and lifecycle reuse | Level 1 | Supplied-map library preparation and shipped-CLI route tests pass, as mapped above. No terminal encoder or keyboard behavior is required by these semantics. |
| Editor completion/hover, released-property treatment, diagnostics, and passive behavior | Level 1 | DMLS request tests and effects instrumentation pass; the area recipe enables instrumentation. |
| Frozen prompts, current schemas, and artifact migration | Level 1; opt-in corpus checks for live internal prompts | Frozen normal-path tests and schema tests pass in the full suites. Cycle 1 recorded the required opt-in prompt checks, including unrelated existing prompt failures; the namespace-migrated clarification prompt was not changed again in this repair. |
| Retained lazy `current` behavior in a real terminal | Level 2 | Fresh `just test-l2 level2_ac28`: seven tests passed through headless tmux, with no selected-test skips. The generator leg selected zero tests, which is not additional terminal evidence. |
| Current documentation and skills explain the implemented contract | Source/artifact review plus Level 1 documentation checks | Updated topic pages describe per-document capture, reuse, source/target boundaries, shell children, model selection, and exact-text refusal; future template approval is marked planned. |

Fresh checks on macOS, through the package recipes and nextest:

- Claudine `just test document_environment`: 18 passed, including all 13 CLI route tests and the task approval tests.
- Darkmatter `just test fm_`: 47 passed, including namespace/editor/passivity checks; this filter also selects unrelated hash tests.
- Darkmatter `just test`: 9,778 passed, 11 skipped; three slow tests as noted above.
- Claudine `just test`: final run 8,738 passed, 9 skipped.
- Claudine and Darkmatter `just lint`: passed.
- Claudine `just test-l2 level2_ac28`: seven passed; real tmux execution, no focus changes.
- Root `just check-tier-coverage claudine` and `just check-tier-coverage darkmatter`: passed, zero stranded tests. New test modules are declared by their consolidated targets and selected by live tiers.

The first Claudine full run, concurrent with Darkmatter's suite, stopped after three failures in the unchanged `sequence_initialize_include_preflight` module. All five module tests passed on a focused rerun, and all passed again in the subsequent complete Claudine run. The initial failure remains recorded here; its cause was not established, and no regression attributable to this feature was reproduced. Claudine also emitted its existing macOS `__eh_frame` linker warning. No implementation source was modified to obtain passing results.

The input robustness matrix does not apply to the production change: it changes expression grammar/lookup and in-memory document environment ownership, not a shipped configuration or file-format loader. The JSON supplied-map reader is a test-fixture seam. Cross-OS evidence remains CI's responsibility and does not determine readiness. This review changes only its output and the requested previous-review/spec frontmatter; it makes no implementation edits, commits, or lifecycle-directory moves.
