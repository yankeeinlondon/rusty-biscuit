---
kind: implementation-log
---

# Test audit: 2026-10-26-task-inputs-and-providers

## Changes and counts

Added **five genuine coverage test functions/Nextest identities** (gross and
net increase both five), strengthened **one existing test**, and modified
**zero slow tests**. Two shared test capture helpers were also strengthened:
they now require an environment and prompt record for every recorded launch
before zipping the records. Missing evidence can no longer silently truncate
the list of observed launches. These helper changes are not additional test
identities. No cases were removed, split, ignored, or moved between tiers.

The new CLI tests cover one confirmed model-ownership defect and its controls:

- `a_retry_drops_a_model_removed_from_its_document` initially failed because
  the second provider received `MODEL=opening-model` after the document had
  removed `model`. An intermediate fix cleared the child environment but the
  same regression still failed because the retried prompt contained the old
  `env.MODEL`. The final fix clears the stale identity from prompt, argv, and
  child environment together.
- `a_rewritten_prompt_drops_its_predicted_model_before_first_launch` proves
  that an earlier task removing a later document's model invalidates that
  prediction before its first launch, including the body environment.
- `a_retry_retains_a_model_supplied_by_with` preserves the task's retained
  input overlay while its document is rewritten.
- `a_retry_retains_a_genuine_enclosing_model_fallback` preserves the sequence's
  provider-compatible fallback when the prompt itself never named a model.
- `a_retry_keeps_the_callers_captured_model` preserves a real captured `MODEL`
  ahead of both the opening document and its fresh read.

The existing `planned_fallback_model_carries_what_the_document_cannot_re_derive`
test now distinguishes document-owned models from genuine inherited models,
review choices, captured environment choices, and provider defaults. Its old
sequence boolean asserted that every frontmatter-selected model should be
retained, which pinned the defect rather than the intended ownership rule.

## Implementation repair

Launch fallback selection now receives the initial effective document hints,
so an own-frontmatter model is re-derived on a fresh read rather than promoted
to immutable invocation intent. Initial task preparation likewise discards a
document-owned prediction when the fresh document removes that model.

The canonical preparation service exposes its existing identity-selection
hook through `prepare_document_selecting`. Harness fresh reads use that hook
after frontmatter resolution and before rendering the body. It delegates to
the same schema-aware composers and shared provider/model selection helpers;
there is no second body composition, shell execution, environment capture, or
separate provider rule. Removing a projected `MODEL` restores the captured
value when one exists. Full launch-plan rebuilding still runs once per
attempt and owns provider argv, process environment, and session compatibility.

Relevant source comments now distinguish fresh document hints from inherited
or reviewed fallbacks. The documentation agent aligned canonical behavior
and the skill summary with this repair. The medium review finding is addressed
by the observed red-before/green-after CLI regression and its controls.

## Requirement coverage

| Acceptance criterion | Boundary and evidence |
| --- | --- |
| 1: task `with`, freed `params`, precedence and unsupported task kinds | Parser/task tests assert typed values, ordinary `seq.params` data, just-in-time evaluation, reserved bindings, caller/runtime precedence, and executable-specific refusal; normal CLI schema tests exercise dispatch. |
| 2: file provenance for task/proxy/retry/prep, eager/lazy/remote/union values and atomic evaluation | `with_file_provenance` runs the real CLI and isolated provider recorder against authoring/target/launch-directory controls. It asserts actual selected file content or native path, missing-file refusal before launch, preserved URLs/raw ambiguous values, anchored forwarding, chained schemas, and no partial launch after an evaluation error. |
| 3: shared mapping error family and locations | Parser/preflight loading tests cover task files and mapping shapes; the initial focused run also selected the shared proxy/retry/prep error-family test. Source locations and whole-mapping precedence are asserted, not just failure status. |
| 4: each task's own provider through repeated passes | Real CLI recording stubs assert binary, argv/stdin, child `AGENT`/`MODEL`, task token ownership, and completion labels over both passes. Claude and Goose are the established protocol-compatible substitutes for the illustrative provider names in the snapshot. |
| 5: provider/model precedence, nested fallback, list hints and model clearing | Preflight/selection unit tests and CLI cases cover caller flags/setters, runtime mutations, nested documents, installed-list filtering, separate model precedence, incompatible provider switches, and cleared stale values. Five new regressions close fresh-read ownership and body/child agreement gaps. |
| 6: unusable static hints fail before any task; non-provider graphs need none | CLI tests assert no recorded spawn, located task/source diagnostics even under false guards, and actual shell/side-effect output without a provider. |
| 7: honest review rows and launched choices | Selection tests key choices by task identity and preserve read-only cells. Five real detached tmux tests exercise layout, model edits, submission to recording executables, read-only own-provider rows, and cancellation before work. |
| 8: late values, captured group state, passive prediction and actual attempt reports | Provider prediction/CLI tests exercise outputs and runtime writes, isolated group-start state, shell substitutions deferred without execution, circular hint errors and overriding sources, unavailable dynamic hints refused before their spawn, and actual retry/proxy identities. Existing skipped/dry-run cases assert no claimed provider. |
| 9: sequence schemas, parser/editor separation and embedded imports | Library/CLI schema suites validate trigger activation, malformed values/keys, both triggers, source-located errors, shipped fixtures, and embedded bundle resolution. Later recursive schema coverage replaces the snapshot's interim bounded schema. The documented scalar `sequence: 42` coercion remains a prior departure: the payload's string arm accepts it and the sequence parser rejects it. |
| 10: rename sweep | Maintained parser/task/fixture tests use `with`; ordinary `params` data remains explicitly tested. No prompt grammar was changed in this slice. Prior opt-in shipped-prompt evidence remains unchanged. |

Internal transformations and scripted provider responses are F1. Real CLI
dispatch, filesystem rereads, and native subprocess argv/environment observations
exercise the actual integration rather than mocking the selector. The L2 tests
cross terminal rendering and input delivery through detached tmux at F2. This
feature does not introduce a new external provider protocol or promise OS key
encoding, so no live provider or L3 evidence was added. Audio stayed suppressed
by the existing isolated CLI fixture; no window was focused.

## Verification

From `claudine/`, the initial scoped parallel L1 run passed **166 tests**,
maximum **3.382 seconds**, before the new regressions. Its filter covered
task providers, provenance, prediction, review, selection UI, schemas, prompt
tasks, shared directive errors, and external task loading. The confirmed
retry regression then failed at the actual child model, and subsequently at
the still-stale body model, before the final implementation passed it.

Final parallel L1, after all changes:

```sh
BISCUIT_TEST_FILTER='test(sequence_task_providers) | test(with_file_provenance) | test(preflight::providers) | test(sequence::review::tests) | test(selection_ui::tests) | test(sequence_schema::) | test(task::tests::prompt_tasks) | test(preflight::tests::loading) | test(harness_orch::loop_control::tests) | test(harness_orch::prompt::tests) | test(target_launch::tests) | test(prepare::service::tests) | test(schema::tests) | test(context_construction_guard) | test(lifecycle_one_implementation_guard)' just test
```

**479/479 passed**, maximum **3.418 seconds**, execution **15.617 seconds**.
The five new tests took **1.127–1.416 seconds**. Compilation is separate from
these per-test timings. Final log: `/tmp/pipeline-s16-final.log`.

After L1 completed, exact-package lint passed:

```sh
cargo clippy -p claudine -p claudine-cli --all-targets --features claudine-cli/test-fixtures -- -D warnings
```

Log: `/tmp/pipeline-s16-clippy.log`.

The initial plan was to reuse unchanged S14 terminal evidence. Because the
repair affects model delivery to launched tasks, the final tree instead reran
only the relevant review-screen tests after L1 and lint completed:

```sh
just test-l2 level2_tmux_review_screen
```

**5/5 passed**, maximum **3.197 seconds**, execution **3.206 seconds**. The
generator recipe leg selected zero matching tests; it did not repeat CLI
observations. This positional filter worked. All panes were detached and
owned by the harness. Log: `/tmp/pipeline-s16-l2.log`.

`git diff --check` passed. No test exceeded five seconds and no slow test
required modification. No full-area suite or other-OS execution is claimed;
the coordinator owns final broader gates. Existing shell-recorder CLI tests
are Unix-only, while the in-process ownership/selection tests remain portable.
No Windows path comparison, platform gate, CI cell, or runner setting changed.
