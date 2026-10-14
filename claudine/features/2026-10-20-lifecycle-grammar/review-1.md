---
$schema: feature-review.yaml
ready: false
findings:
    - "high: Mixed-text expressions bypass validation in unselected branches"
observations:
    - Existing opt-in prompt failures remain separate work
human_review: false
human-in-the-loop: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-13T19:38:51-07:00
spec: 2026-10-20-lifecycle-grammar/spec.md
implemented: true
implemented_by: claude/opus
log: claudine/features/2026-10-20-lifecycle-grammar/log.md
description: "A **feature** review of `2026-10-20-lifecycle-grammar/spec.md`"
feature: 2026-10-20-lifecycle-grammar/review-1.md
next: 2026-10-20-lifecycle-grammar/review-2.md
---

# Lifecycle Grammar — Review 1

The feature is **not production ready**. One validation requirement remains
unmet: malformed expressions inside mixed text are accepted in unselected
branches. The new ordering, conditional execution, retry overlays, audio queue
publication, and editor path handling otherwise matched the contracts examined.

## Findings

### High: Mixed-text expressions bypass validation in unselected branches

**Authority: spec, “Shape validation and branch execution”:** “Parse and validate
both branches before execution, including branch shape, verb placement,
expression syntax, nested-span checks, and shell approval.” This applies to
every stack, including task setup/teardown. A plausible authoring typo such as
`status {{ fm. }}` is invalid expression syntax, even when its branch is not
selected.

**Defect class:** mixed-text action values retain their interpolation spans
without checking those spans' syntax during static validation.

In Claudine, [action_value_to_expr](../../lib/src/composition/lifecycle/action_shape.rs)
parses a whole-value expression but returns mixed text as a string literal.
The later [validate_no_err_in_no_error_events](../../lib/src/composition/lifecycle/validate.rs)
check attempts to prepare that text, then silently continues when preparation
fails. Consequently, walking both branches does not establish that both
branches have valid expression syntax. The recursive `set` and overlay readers
use the same action-value conversion.

Reproduction, using the branch structure exercised by Claudine CLI's
[branch-selection fixture](../../cli/tests/l1/lifecycle_set_shell_values.rs):

```yaml
---
pick: else
start:
  - when: "pick == 'then'"
    then:
      - info: "status {{ fm. }}"
    else:
      - info: "selected else"
---
Body.
```

Run `claudine compose --goose prompt.md` with a recording stub provider. The
command exits **0** and launches the provider. Replacing the malformed value
with `"{{ fm. }}"` exits **1**, reports an invalid lifecycle action, and does
not launch it. The mixed-text case should also fail before provider launch,
naming `start[0].then[0].info.text`. Otherwise a document can pass preparation
until a different branch is selected in a later run.

The sweep used an isolated temporary home, working directory, and provider
stub; audio was disabled. It tested malformed mixed text in both an unselected
`then` and an unselected `else`. Every affected row below contains the same
`status {{ fm. }}` typo; the control rows distinguish correct rejection from
simply skipping all validation.

| Site or reader | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Communication action reader | Short and long forms of `say`, `speak`, `effect`, `message`, `notify`, `stderr`, `stdout`, `info`, `warn`, and `success` | Exit 0; provider launched for every verb and form | Reject malformed span before launch |
| Control parameter reader | `error.reason`, `proxy.target`, `retry.max_attempts`, `retry.delay`, `resume.message`, `resume.max_attempts`, `defer.delay`, and `defer.reason` | Exit 0; provider launched for every parameter | Reject malformed span before launch |
| Recursive runtime mutation reader | `set.note` and nested `set.meta.notes[0]` | Exit 0; provider launched | Reject malformed span before launch |
| Recursive overlay reader | Scalar and nested array/object values in both `proxy.with` and `retry.with` | Exit 0; provider launched | Reject malformed span before launch |
| Side-effect positional reader | `set_frontmatter`, `merge_frontmatter`, `delete_frontmatter`, `increment_frontmatter`, `decrement_frontmatter`, `append_frontmatter`, `prepend_frontmatter`, `ensure_file`, `ensure_dir`, `append_line`, `append_jsonl`, and `http_post` | Exit 0; provider launched for every verb | Reject malformed span before launch |
| Side-effect named-parameter reader | `ensure_file.content` | Exit 0; provider launched | Reject malformed span before launch |
| Shared expression-function reader | `length` in positional and named-parameter forms | Exit 0; provider launched in both forms | Reject malformed span before launch |
| Shell preflight reader — clean | Mixed text in `shell.command` and `shell.on_error` | Exit 1; malformed span reported; provider never launched | Same |
| Guard parser — clean | Nested `when: "fm."` in an unselected branch | Exit 1; provider never launched | Same |
| Whole-value action parser — clean | `info: "{{ fm. }}"` in an unselected branch | Exit 1; provider never launched | Same |

The communication, control, side-effect, and expression-function rows share
the short-form/named-parameter action-value reader; `set` and `with` add
recursive traversal around it. Zero-argument directives and literal enum/flag
parameters do not read interpolation operands and are not sibling readers.

The same malformed `info` leaf was also tested in every stack home and at each
command boundary:

| Stack or command site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| `initialize`, `start`, `blocked`, `success`, `failure`, `finalize` | Each event contains an unselected conditional branch | Exit 0; provider launched in all six cases | Reject syntax before launch regardless of event selection |
| `loop.gate` | Unselected conditional branch; `while: "false"` | Exit 0; provider launched | Reject syntax before launch |
| Task `setup` and `teardown` | Referenced task file with an unselected conditional branch | Sequence exits 0; child provider launched in both cases | Reject syntax before task execution |
| `compose` | Reproduction above | Exit 0; provider launched | Reject syntax before launch |
| `inline-compose` | Same stack plus a valid `prompt` field | Provider launched; later exits 1 because the stub does not update the document | Reject malformed lifecycle syntax before launch |
| `sequence` | Same stack plus a valid child prompt step | Exit 0; child provider launched | Reject syntax before launch |

**Recommended repair:** parse the expressions in every mixed-text operand
during preparation, without evaluating them. Reuse the shared value preparation
or expression parser rather than adding another interpolation grammar. Preserve
the authored operand path in the error, and cover the recursive `set`/overlay
readers and task stacks in the same repair. Add Level-1 tests showing that a
malformed span in either unselected branch is rejected, while a syntactically
valid operand that would fail only during evaluation remains unevaluated there.
This is a bounded validation repair; an escaping layer or a new syntax is
unnecessary.

## Verification and coverage

These checks were run during this review with Nextest through the package
recipes. All selected tests passed; passing existing tests does not cover the
finding above.

| Check | Result |
| --- | --- |
| Claudine `just test-library lifecycle` | 620 passed |
| Claudine `just test-cli compose_removed_grammar` | 29 passed |
| Claudine `just test-cli overlay_layering` | 31 passed |
| Claudine `just test-cli lifecycle_set_shell_values` | 9 passed |
| Darkmatter `just test nested_span` | 21 passed, including DMLS path and schema inventory tests |
| Darkmatter `just test host_globals` | 8 passed |
| Claudine `just lint-lifecycle-doc-facets` | Passed |

Verification levels were assessed against the requirements, rather than test
counts:

| Requirements | Appropriate level and evidence present |
| --- | --- |
| Stack shapes, nesting limit, list-local control ordering, flags, authored paths, and retired grammar (acceptance criteria 1–4, 6, and 17) | Level 1: parser tests plus CLI rejection tests. Mixed-text syntax validation is the missing case described above. |
| Retry overlay evaluation, fresh reread, schema validation, one-transition lifetime, resume clearing, and retained proxy overlays (criterion 5) | Level 1: executor and orchestration tests, plus the recording-provider CLI test. The orchestration tests assert the resulting prompt and effective values. |
| Loop globals and distinction from `fm.loop` (criterion 7) | Level 1: binding, loop engine, CLI ambient-value, and Darkmatter host-global tests. |
| Ordered durable audio publication and no publication during dry run (criterion 8) | Level 1: fake emitter and locked-spool tests. This contract concerns publication, not audible playback; the tests appropriately avoid real sound and TTS providers. |
| Lift-ready dependency boundary (criterion 9) | Level 1: six source guards and a guard self-test. |
| Migrated prompt parsing, selected-branch execution, and route fixture consistency (criteria 10–11) | Level 1: frozen-artifact tests and recording-provider tests; live internal corpus and route-drift checks are opt-in and have passing results recorded in the implementation log. |
| Editor extension loading, nested paths, pointer mapping, and mapping corpus (criterion 12) | Level 1: schema and DMLS diagnostic/pointer tests. No OS input behavior is introduced. |
| Documentation, skill migration, and residue inventory (criteria 13–14 and 16) | Static checks and the implementation log's classified inventory. |
| Package tests, real-terminal checks, lint, and informational prompt checks (criterion 15) | The implementation log records passing final package L1/lint checks and 282 CLI plus 3 generator Level-2 tests using tmux/WezTerm. Four Kitty tests were not run. This review reused those recorded checks and ran the focused checks above; it did not rerun the full suites or terminal tier. |

The grammar introduces no keyboard, mouse, paste, or input-encoder contract
requiring Level 3. No additional verification-level mismatch was found.
The reviewed new L1 modules are declared by their consolidated targets, and
the lifecycle unit modules are compiled by the library test target. CLI
terminal tests have a declared target and live `test-l2` recipe; live prompt
tests retain their explicit opt-in target. No tests were added or renamed by
this review.

The configuration input robustness matrix does not apply: the changed parser
implements the Markdown lifecycle action grammar. The spec explicitly defines
its empty-stack forms and retains the existing frontmatter decoder's duplicate
key handling.

## Observations

### Existing opt-in prompt failures remain separate work

The implementation log records eight existing opt-in prompt execution/contract
failures and 31 non-grammar prompt schema-validation failures. It separately
records passing lifecycle corpus and route-drift checks. These failures are
reported as predating this feature, and this review did not independently
rerun or establish their origin. Keep them visible for the author to schedule;
they are not additional findings against this grammar change.

## Review closure

No important unresolved design choice or deferred in-scope behavior requires
human review. The logged retry-overlay consumption adjustment preserves the
specified one-transition lifetime across preflight and materialization; prompt
migration ordering follows the spec's canonical order. The finding requires
implementing the existing syntax-validation contract, not changing it.

The spec's `review_iterations` is now 1. It is not marked completed because
this review contains a high finding. No next spec or plan was supplied, so
there is no downstream document to refresh. The feature remains in its active
directory for the fix and review cycle.
