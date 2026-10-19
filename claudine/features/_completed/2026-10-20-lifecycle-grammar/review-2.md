---
$schema: feature-review.yaml
ready: true
findings: []
observations:
    - Existing opt-in prompt failures remain separate work
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-13T20:09:36-07:00
spec: 2026-10-20-lifecycle-grammar/spec.md
implemented: false
description: "A **feature** review of `2026-10-20-lifecycle-grammar/spec.md`"
feature: 2026-10-20-lifecycle-grammar/review-2.md
previous: 2026-10-20-lifecycle-grammar/review-1.md
---

# Lifecycle Grammar — Review 2

The feature is **production ready** within the specification's scope. The
previous blocking finding is repaired, including its recursive readers and
task stacks. No new high, medium, or low findings were identified. No human
design decision is required to close this review.

## Previous unblocked findings

[Review 1](review-1.md) contains one unblocked high finding, “Mixed-text
expressions bypass validation in unselected branches,” under `## Findings`.
Its authority was the spec's requirement to validate expression syntax in
both branches before execution. The repair satisfies that requirement.

In the Claudine package,
[action_value_to_expr](../../lib/src/composition/lifecycle/action_shape.rs)
now parses every interpolation span in mixed text without evaluating it.
The existing short-form, named-parameter, and recursive readers all reach
this conversion. Malformed operands fail before execution, with their authored
operand paths. Valid expressions remain stored as authored templates and are
evaluated only when their action runs.

The repair's [reader sweep](../../lib/src/composition/lifecycle/tests/mixed_span.rs)
was inspected and executed. It checks 54 reader rows in both branch positions,
then checks every stack home. The following table records the disposition of
every reader class reported previously; these are resolved instances, not new
findings.

| Reader or boundary | Verification and result |
| --- | --- |
| Communication operands | All ten spellings, including `speak`, reject malformed mixed spans in short and long form, naming `.text`. |
| Control operands | Short forms and named `reason`, `target`, `max_attempts`, `delay`, and `message` parameters reject malformed spans. |
| Runtime `set` values | Scalar and nested object/array values reject malformed spans at their nested authored paths. |
| `proxy.with` and `retry.with` | Both scalar and nested values reject malformed spans through the shared recursive reader. |
| Side-effect operands | All twelve side-effect verbs reject malformed positional operands; named `ensure_file.content` also rejects them. |
| Expression-function operands | Positional and named `length` arguments reject malformed spans through the same conversion used by catalog functions. |
| Previously clean controls | Whole-value operands, guards, and shell operands retain syntax validation. The shared conversion now checks mixed shell operands before approval as well. |
| Stack homes | Library tests reject malformed spans in all six events, loop gates, loose loop verbs, and task setup/teardown. |
| Command boundaries | Nine CLI tests prove rejection before provider launch in `compose`, `inline-compose`, `sequence`, loop gates, and referenced task setup/teardown; positive controls launch successfully. |
| Lazy evaluation | Executor and CLI controls accept syntactically valid expressions that would fail during evaluation in an unselected branch. Selecting the failing operand produces an evaluation error. |

The repair also updates tests that previously expected malformed mixed text
to survive preparation, and corrects the corresponding lifecycle documentation
and skill description. Sharing the positional field-name lookup with the
source map keeps error paths consistent without a second interpolation parser.

## Unblocked Findings

None. The parser, execution ordering, retry-overlay lifetime, loop bindings,
audio publication, migration, and editor integration examined match the
specified contracts. No additional performance or ergonomics change is needed
for readiness.

## Verification and requirement coverage

The following checks were run during this review. All selected tests passed;
the numbers describe these runs, not the entire package suites.

| Check | Result |
| --- | --- |
| Claudine `just test-library lifecycle` | 626 passed |
| Claudine `just test-library composition::prepare::tests` | 58 passed |
| Claudine `just test-cli lifecycle_mixed_span_validation` | 9 passed |
| Claudine `just test-cli compose_removed_grammar` | 29 passed |
| Claudine `just test-cli overlay_layering` | 31 passed |
| Claudine `just test-cli lifecycle_set_shell_values` | 9 passed |
| Darkmatter `just test nested_span` | 21 passed, including DMLS classifier and schema inventory checks |
| Opt-in Claudine CLI prompt target, selecting the lifecycle corpus and `shipped_prompt_route_drift::` | 5 passed; no hash-update flag |
| Claudine `just lint-lifecycle-doc-facets` | Passed |
| Repository `just check-tier-coverage claudine` | Zero stranded tests |

Verification levels were assessed against each requirement:

| User-facing requirement | Appropriate level and evidence |
| --- | --- |
| Lists, bundles, canonical ordering, nesting limit, list-local directive placement, flags, aliases, unknown keys, and authored paths (acceptance criteria 1–4 and 17) | Level 1: grammar, validation, and executor tests, including both-branch shell collection and initialization prohibitions. The focused lifecycle run passed. |
| Retry overlays: source-side evaluation, native values, fresh read before schema validation, one-transition lifetime, resume clearing, retained proxy overlay, and no disk writes (criterion 5) | Level 1: retry executor tests and orchestration tests asserting effective frontmatter and resulting prompts. The focused lifecycle and overlay runs passed; the existing recording-provider test supplies the CLI boundary. |
| Retired grammar, replacement hints, literal-data exemptions, and rejection through all three commands (criterion 6) | Level 1: parser and CLI rejection tests, including render-time discovery of retired loop reads. The focused runs passed. |
| Loop object values, bare/dotted/bracket access, `fm.loop`, scope errors, and no frontmatter fallback (criterion 7) | Level 1: binding and loop tests, migrated CLI ambient-value tests, and Darkmatter host-global tests. Existing passing package evidence is recorded in the implementation log. |
| Ordered durable audio publication, survival across transitions, and no publication in dry run (criterion 8) | Level 1: fake publishers and locked-spool tests. The contract concerns publication rather than audible playback; the focused lifecycle run passed without playing audio. |
| Parser dependency boundary (criterion 9) | Level 1: source guards and their self-test, included in the passing lifecycle run. |
| Migrated prompt parsing, selected-branch effects in order, and route fixture consistency (criteria 10–11) | Level 1: frozen fixtures, recording-provider branch tests, and opt-in live corpus/route checks. The affected checks passed again with the repaired parser. |
| Extension loading, editor path classification, pointer mapping, and diagnostic mapping corpus (criterion 12) | Level 1: Darkmatter/DMLS tests. The focused nested-span checks passed; the unchanged loading, pointer, and mapping-corpus checks have passing package evidence in the implementation log. |
| Documentation, skill migration, and classified residue inventory (criteria 13–14 and 16) | Static checks and document inspection. The documentation gate passed; the skill search excludes only historical `timeline.md` and returns no forbidden spellings. The migration search matches the implementation log's intentional categories. |
| Package tests, lint, terminal tier, and inspected prompt results (criterion 15) | Reused the recorded full-suite and lint checks: the repair log reports Claudine 8885 passed and lint passing; the implementation log reports Darkmatter 9950 passed and lint passing, plus 282 CLI and 3 generator Level-2 tests. Focused checks above were rerun against the working tree. |

The recorded Level-2 tests exercise terminal rendering through tmux and
WezTerm without taking focus. Four Kitty drain-interruption tests were excluded
from the earlier run because that host setup would open a visible window;
they do not read lifecycle grammar. This review did not rerun terminal tests
or the full package suites. The grammar introduces no keyboard, mouse, paste,
or input-encoder behavior requiring Level 3, and no requirement lacks an
appropriate verification level.

The new library test module is compiled by the library's unit-test target.
The new CLI module is declared by the consolidated `l1` target and selected
by the live L1 recipe. Its recording-provider fixture is Unix-only, while
the parser and executor coverage is portable; this does not change the
production implementation's platform support. The prompt target is explicitly
opt-in and was run with its required feature. No tests were added or renamed
by this review.

The configuration input robustness matrix does not apply: this change reads
the Markdown lifecycle action grammar, which the review instructions explicitly
exclude. Empty-stack and branch-body shapes are specified and tested; the
existing frontmatter decoder retains ownership of duplicate YAML keys.

## Recurrence

No finding recurs. Review 1's mixed-text syntax-validation class is closed
across the listed readers and stack boundaries, so `recurrence` is `false`.

## Observations

### Existing opt-in prompt failures remain separate work

The implementation log records eight unrelated prompt execution/contract
failures and 31 non-grammar prompt schema-validation failures from the root
informational prompt check. This review reran the affected lifecycle corpus
and all four route-drift checks successfully, but did not rerun those unrelated
failures or independently establish their origin. They remain visible for the
author to schedule and do not require a repair in this grammar review cycle.

## Review closure

Review 1 is marked implemented and links to this review. The spec now records
two review iterations and `completed: true`. The feature remains in its
active directory for the author's review-cycle closure. No code was changed,
no commit was created, and no feature directory was moved.
