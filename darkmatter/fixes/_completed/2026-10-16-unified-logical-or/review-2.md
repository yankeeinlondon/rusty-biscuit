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
created: 2026-10-30T21:20:26-07:00
spec: 2026-10-16-unified-logical-or/spec.md
implemented: false
description: "A **fix** review of `2026-10-16-unified-logical-or/spec.md`"
fix: 2026-10-16-unified-logical-or/review-2.md
previous: 2026-10-16-unified-logical-or/review-1.md
---

# Review 2: Unified Logical OR

The fix is **production ready**. The previous finding is verified fixed, including every required instance in the repair ledger. No new findings or human decisions are required.

The review read the specification, previous review, repair log's Finding Ledger, and original implementation log; inspected the repair diff and the parser, evaluator, call checker, lifecycle tests, absence diagnostics, and documentation relevant to the acceptance criteria. The repair changes test coverage rather than production behavior.

## Previous Finding Verification

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: Shipped-condition corpus does not check passive call-check compatibility | R1-F1.1 through R1-F1.6 below | verified fixed | The corpus now compares rejected authored calls between the unified syntax tree and a test-only projection into the former boolean `or(...)` representation. The comparison and its controls passed independently during this review. | Nothing |
| R1-F1.1: Bare lifecycle conditions | Every extracted `when`, `while`, and `until` value | verified fixed | `ConditionKey` is included in `CONDITION_SURFACES`; recursive frontmatter extraction feeds the comparison with lifecycle declarations. The passing gate requires every extracted condition to parse and be compared, and requires this surface to contain logical OR. | Nothing |
| R1-F1.2: Directive and shell-ternary conditions | Directive `when=`, shell-ternary conditions, and schema-expression conditions | verified fixed | All three surface categories are included. Directive extraction uses the shared directive scanner; existing shell-ternary and schema-expression extraction controls passed. The gate compares every present condition, rather than silently accepting a parse failure. | Nothing |
| R1-F1.3: Claudine frontmatter conditions | Cover the Claudine corpus once through the shared Darkmatter corpus, including declarations and host-scope assessment | verified fixed | Compared the two extractors: Darkmatter recursively reads the same lifecycle keys and walks the root prompt directory plus three additional directories. Claudine's contract now points to the shared gate. Both corpus tests passed. The scope limitation is assessed below. | Nothing for the shipped conditions |
| R1-F1.4: Claudine body block conditions | Cover body conditions once through the shared corpus | verified fixed | Darkmatter's directive scanner covers the `::block when=` conditions inspected by Claudine's line-based extractor, as well as other directives. The shared gate and Claudine contract both passed. | Nothing |
| R1-F1.5: Public call checker distinguishes a rejected call | Original condition and the edited `length(true)` condition; a control that detects changed compatibility | verified fixed | `call_check_distinguishes_a_rejected_condition_call` retains the original/edited distinction. `call_verdict_comparison_reports_a_changed_verdict` detects the former rejection of `length(items || []) > 0`. `projection_marks_only_synthesized_or_calls` preserves authored `or(...)` calls and spans. All three passed. | Nothing |
| R1-F1.6: Acceptance mapping | Correct the implementation log's acceptance-criterion 3 evidence | verified fixed | The acceptance roll-up now identifies the old corpora as syntax-only and separately names the new comparison and all three controls. | Nothing |

The repair's [comparison and projection](../../lib/tests/prompts/dasherized_identifier_corpus.rs:564) use the same rejection predicate as composition, compare function names and authored spans, and exclude only synthesized `or(...)` calls. The projection exhaustively visits the syntax-node variants, including callback bodies, so nested logical OR is not omitted. The comparison has a discriminating negative control; passing is not simply agreement between two equivalent parser calls.

The [scope builder](../../lib/tests/prompts/dasherized_identifier_corpus.rs:667) deliberately leaves Claudine-specific host globals untyped and omits enclosing-guard narrowing. This was already disclosed in review 1's repair decision and needs no renewed approval. I inspected the shipped conditions containing `||`: their results are consumed as truth tests, through final conditions, negation, or boolean conjunction, rather than passed as values to functions or comparisons. Their authored function arguments therefore do not acquire a different type from the changed operator result. Both logical-OR representations also supply the same false-side facts to later operands. These limitations do not conceal a compatibility change in the current shipped conditions; reproducing Claudine's entire host analysis in this test would add machinery without protecting an additional current case.

## Unblocked Findings

None.

## Blocked Findings

None.

## Acceptance and verification

All required behavior is expression evaluation, composition, passive analysis, or lifecycle execution. Level 1 is the appropriate verification level: no acceptance criterion depends on a terminal emulator, keyboard encoding, or OS keyboard injection.

| Requirement | Verification level and evidence | Result |
| --- | --- | --- |
| Acceptance criterion 1: deciding operands and short-circuiting on every surface | Level 1: public evaluator value cases, interpolation and page-block composition regressions, lifecycle guard value table | Passed |
| Acceptance criterion 2: open-questions guard fires only for a populated list | Level 1: lifecycle execution with temporary plan files, present and absent cases | Passed |
| Acceptance criterion 3: condition outcomes and passive verdict compatibility | Level 1: parser shape tests, logical-OR regression tests, repaired shipped-condition comparison and negative controls | Passed; R1-F1 closed |
| Acceptance criterion 4: remove mode APIs and compile every feature | Source sweep plus `cargo check --workspace --all-features --all-targets --color=never` | Passed; the remaining Claudine loop-configuration function named `parse_condition` is unrelated to the removed expression API |
| Acceptance criterion 5: only the left bare operand handles absence | Level 1: page-block advisory tests; inspected the corresponding DMLS diagnostics and shared absence behavior | Passed |
| Acceptance criterion 6: nested-span rewrite suggestion | Level 1: lint and reparse tests for logical OR | Passed |
| Preserve boolean `&&`, `or(...)`, and `and(...)` | Level 1: expression parser suite and operator parity tests; inspected unchanged evaluation contract | Passed |

Commands run during this review:

- `cargo nextest run -p darkmatter --features prompt-tests --test prompts --color=never`: 14 passed, including the repaired comparison and all controls; the longest test took 1.625 seconds.
- `just test logical_or` in Darkmatter: 21 passed, including composition, advisory, lint, and language-server tests.
- `just test when_logical_or` in Claudine: 3 passed, including the populated and absent open-questions cases.
- `cargo nextest run -p simplified-schema --color=never -E 'test(expression::)'`: 264 passed.
- `cargo nextest run -p claudine-cli --features prompt-tests,test-fixtures --test prompts --color=never shipped_prompts_have_parseable_schemas_and_expressions`: 1 passed.
- `cargo check --workspace --all-features --all-targets --color=never`: passed.

The new tests belong to the explicitly declared `prompts` target and run with its `prompt-tests` feature. Keeping the live-prompt corpus opt-in follows repository policy. None of the new test names accidentally selects a different tier. This review did not rerun the full area suites or lint gates already recorded by the implementer. Some builds repeated the existing macOS linker warning about a large unwind section; no verification failed. Cross-OS execution remains CI's responsibility and is not a readiness condition here.

The current expression documentation describes one grammar, operand-returning logical OR, and boolean conjunction and explicit boolean functions. No remaining mode split was found in the inspected current references. The input robustness matrix does not apply: this work changes a text grammar and its consumers, not a configuration loader or file-format validator.

## Recurrence

No recurring finding. R1-F1 is closed; no unchanged instance, repair regression, or missed sibling was found. The previous review's informational decisions remain accepted without reopening them.
