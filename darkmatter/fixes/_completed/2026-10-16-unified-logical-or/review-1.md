---
$schema: feature-review.yaml
ready: false
findings:
    - "R1-F1 (high): Shipped-condition corpus does not check passive call-check compatibility"
human_review: true
human_review_items:
    - "Optional confirmation: keep the six unrelated, pre-existing Claudine prompt-test failures outside this fix; the recommendation is to retain the recorded baseline."
    - "Optional confirmation: accept validation scoped to migrated crates during intermediate phases; later phases completed the whole-area checks."
    - "Optional confirmation: keep the public schema-validation test that replaced a comparison between two parsers that are now identical."
    - "Optional confirmation: keep renamed tests that describe the unified grammar and the newly required nested-span suggestion."
    - "Optional confirmation: keep list-layout checks on the side-effects report after the expressions report lost its list, and omit the duplicate margin test."
    - "Optional confirmation: keep the language server's right-operand warning and authored logical-OR hover, as required by the spec's shared absence rule and syntax tree."
    - "Optional confirmation: keep the expression-format reference in its current SimplifiedSchema documentation location rather than duplicating it in Darkmatter."
    - "Optional confirmation: keep removal of callback assertions that would otherwise invoke the same parser twice."
    - "Optional confirmation: accept direct implementation of small tasks and serial final checks instead of additional implementation agents."
    - "Optional confirmation: keep the additional lifecycle test covering all seven acceptance-table rows."
    - "Optional confirmation: keep the new tests in existing declared Level-1 modules."
    - "Optional confirmation: accept skipping redundant whole-suite runs in the test-only phase; final validation ran them afterward."
    - "Optional confirmation: leave cross-platform execution to CI; this review does not make it a readiness condition."
    - "Optional confirmation: accept searching the real skill directory because the alternate directory is a symbolic link."
    - "Optional confirmation: leave historical research and already-correct documentation unchanged, including the skill's still-accurate fallback examples."
    - "Iteration 1 repair decision: the shipped-condition call-check comparison types Claudine host globals as unknown and applies no enclosing-guard narrowing, because neither can be built passively from Darkmatter; both trees share that scope. Alternative: a second, Claudine-side comparison."
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-30T21:03:10-07:00
spec: 2026-10-16-unified-logical-or/spec.md
implemented: true
description: "A **fix** review of `2026-10-16-unified-logical-or/spec.md`"
fix: 2026-10-16-unified-logical-or/review-1.md
log: /Volumes/coding/wt/rusty-biscuit/feat-schema-narrowing-and-extraction/darkmatter/fixes/2026-10-16-unified-logical-or/log.md
implemented_by: claude/opus
next: 2026-10-16-unified-logical-or/review-2.md
---

# Review 1: Unified Logical OR

The fix is **not production ready** because the shipped-condition compatibility check required by acceptance criterion 3 is missing. The reviewed operator behavior, parser migration, and targeted tests passed. No runtime defect was reproduced. No finding needs a human decision before repair.

The review covers the implementation commits `c5e6fe1ba` and `97eca4f6f`, documentation commit `ea8c6f1ec`, the specification, plan, and implementation log. Two independent agents read the decisions and deferred work in parallel; their inventories agreed.

## Findings

### R1-F1 (high): Shipped-condition corpus does not check passive call-check compatibility

**Authority — spec, acceptance criterion 3:** “Every shipped condition keeps its passive call-check verdict, checked by the passive shipped-artifact corpus test.” The implementation log cites the parsing corpora as satisfying this requirement, but neither corpus calls the passive call checker or compares its results with the previous behavior.

**Defect class:** a compatibility test checks whether expressions parse while omitting the later decision that can refuse their function calls before execution.

This distinction matters because conditions now produce an operand-returning syntax node where they previously produced a boolean-returning function call. A condition can still parse, and even have the same final truth value, while the checker assigns a different argument type to a nested call. The spec explicitly requires checking that shipped conditions retain their verdicts; runtime truthiness tests and the search for value-consuming conditions do not establish that result.

The affected code is in Darkmatter's [corpus parser](../../lib/tests/prompts/dasherized_identifier_corpus.rs:307) and Claudine CLI's [shipped-prompt contract](../../../claudine/cli/tests/prompts/shipped_prompt_contract.rs:165). The first discards the parsed tree; the second checks schema resolution and expression syntax. Neither retains a call-check result.

**Reproduction and bounded sweep.** Both named corpus tests pass on this tree. For a discriminating control, I used the shipped condition `!total_phases || total_phases <= 0` from `prompts/_implement/implement-plan.md`, then changed only its expression to `length(true) > 0 || (!total_phases || total_phases <= 0)`. Through Darkmatter's public APIs, both parse successfully; the original has no rejected call, while the edited expression has a rejected `length` call. A temporary Level-1 probe asserted those results and passed; its source was removed afterward. This demonstrates the missing distinction without executing the prompt or changing helper behavior.

The same original/edited expression pair was checked against every parsing branch used by the two cited corpora. Their source makes the outcome deterministic: each branch accepts any expression for which the shared `parse` succeeds. The table includes their final pass/fail decisions rather than stopping at the parser.

| Child ID | Site | Shape checked | Observed result | Required result |
| --- | --- | --- | --- | --- |
| R1-F1.1 | Darkmatter `frontmatter_expressions` → `parses` → `gate_b_failures` → `shipped_corpus_expressions_all_parse` | Bare lifecycle `when`/`while`/`until` condition; original and edited pair | Both pass the expression gate; the parsed tree is discarded | Shipped conditions have their passive call-check verdict compared with the baseline |
| R1-F1.2 | Darkmatter `body_expressions` and shell-ternary extraction → the same corpus gate | Directive `when` and shell-ternary condition text; same pair | Both pass; surface classification does not change the parse-only gate | The comparison includes these condition surfaces |
| R1-F1.3 | Claudine CLI `inspect_frontmatter_value` → `shipped_prompts_have_parseable_schemas_and_expressions` | Bare lifecycle condition or interpolated condition text; same pair | Both expression forms pass syntax inspection; schema loading does not call the checker | Check authored calls under the condition's effective declarations and host scope |
| R1-F1.4 | Claudine CLI `inspect_block_conditions` → the same contract test | Body block condition text; same pair | Both pass syntax inspection | Include body conditions in the passive comparison, or cover them once through the shared Darkmatter corpus |
| R1-F1.5 | Darkmatter public `check_expression_calls` | Same pair, with the same untyped declarations | Clean: original has no rejected call; edited expression rejects `length(true)` | Preserve this distinction when the corpus judges conditions |

The adjacent synthetic tests in `design_time_call_check.rs`, `compose_call_precheck.rs`, and DMLS's `call_check_agreement.rs` do check real call verdicts. Their targeted runs pass. They do not walk the shipped conditions or compare those conditions with their former interpretation, so they cannot close this acceptance gap. Claudine's separate nested-span corpus checks literal interpolation mistakes, not argument compatibility.

**Recommended repair:** extend one existing opt-in shipped-artifact corpus to compare passive call-check results for each shipped condition under its actual declarations and host scope. Compare authored function calls, accounting for removal of the old synthetic `or` call. A small test-only projection of condition `LogicalOr` nodes into their former boolean `or` representation can provide the reference without restoring the deleted production API; alternatively, capture explicit expected verdicts from the base implementation. Keep a negative control proving the assertion detects a changed verdict. Cover all condition extractors once and update the implementation log's acceptance mapping. No production compatibility layer or additional process-spawn matrix is needed.

## Acceptance and verification

All behavior in this fix is expression parsing, evaluation, passive analysis, or report content. Level 1 is appropriate for the operator requirements; they do not depend on terminal input encoding, glyph widths, or keyboard events. The existing Level-2 report-layout tests were inspected but not rerun; this fix specifies no new terminal-layout contract.

| Requirement | Strongest relevant verification | Result |
| --- | --- | --- |
| Same deciding operand, truthiness, and short-circuiting across interpolation, block conditions, and lifecycle guards | Level-1 public evaluator, composition, and lifecycle executor tests | Pass; shared evaluator owns the exhaustive operand cases and surface tests exercise wiring |
| Plan open-questions guard fires only when questions exist | Level-1 lifecycle executor reading a temporary plan file | Both present and absent cases pass |
| Existing condition and static-analysis results remain compatible | Level-1 condition and call-check tests; shipped corpora currently check syntax | Synthetic tests pass; required shipped passive comparison missing, R1-F1 |
| Removed parse-mode APIs and every-feature workspace compilation | Source sweep and `cargo check --workspace --all-features --all-targets --color=never` | Pass; Claudine's remaining local `parse_condition` parses a loop configuration map and is unrelated |
| Only the left bare operand handles absence | Level-1 composition warnings and language-server diagnostics | Reviewed tests implement the required distinction; targeted logical-OR tests pass |
| Nested-span lint offers a suggestion containing logical OR | Level-1 lint and rewrite tests | Pass |
| Boolean `&&`, `or()`, and `and()` remain unchanged | Level-1 parser, evaluator, and existing call tests | Pass |

Review-time commands and outcomes:

- `just test expression` in simplified-schema: both recipe passes succeeded, including 288 filtered tests and the recipe's 1,112-test run.
- `just test logical_or` in darkmatter: 21 passed.
- `just test expression` in darkmatter: 1,128 passed.
- `just test call` in darkmatter: 472 passed.
- `just test when_logical_or` in claudine: 3 passed.
- Darkmatter's opt-in prompts binary: 10 passed.
- Claudine CLI's opt-in `shipped_prompts_have_parseable_schemas_and_expressions`: 1 passed.
- Temporary public-API distinction probe: 1 passed; source restored.
- Every-feature, every-target workspace check: passed.

Some test builds repeated the existing macOS linker warning about the large unwind section. No test failed. The implementation log also records final area suites and lint checks; those full gates were not repeated here. Cross-OS evidence is left to CI and does not affect this review's readiness decision.

The input robustness matrix does not apply: this fix changes a text expression grammar and its consumers, not a file-format or configuration loader. The schema-format change selects the same unified parser without introducing new fields or document-loading rules.

## Implementation decisions (informational)

Every decision below can proceed without a human ruling. The recommendation is to retain each choice. These are optional author checks after the repair cycle, not blocked findings.

| Decision or deferred work | Assessment |
| --- | --- |
| Leave six pre-existing Claudine opt-in prompt failures unchanged | Recorded before implementation and unchanged afterward; prompt rehearsal and fixture maintenance are unrelated to logical OR. This does not excuse R1-F1, which concerns a different, explicitly required assertion. |
| Validate only migrated crates in intermediate phases | The plan explicitly expects downstream compilation to recover in later phases. Final area gates were subsequently completed; no validation remains deferred for this reason. |
| Replace the obsolete two-parser comparison with public expression-format validation | Retains the meaningful contract and avoids comparing a parser result with itself. |
| Rename tests describing removed modes | Cheap internal naming changes; the changed nested-span expectation is explicitly required. |
| Move list-layout assertions to the side-effects report and remove the duplicate margin test | The expressions report no longer contains that list. Existing terminal coverage still exercises the list behavior where a list exists. |
| Update the language server's right-operand warning and authored logical-OR hover | Follows the spec's absence rule and syntax-node decision; no new author choice is required. |
| Update expression-format documentation where it now lives | Keeps one current reference in SimplifiedSchema rather than recreating moved content in Darkmatter. |
| Remove duplicate callback assertions using the deleted second parser | The retained assertions exercise the one grammar; duplicate calls would add no verification. |
| Implement small phase tasks directly and run final checks serially | Internal workflow choice with no public behavior consequence. |
| Add a lifecycle test covering every acceptance-table row | Useful verification of the third required surface. |
| Add tests within existing declared Level-1 modules | The tests compile and are selected by live recipes; no stranded test was found. |
| Omit redundant full-suite runs during the test-only phase | Final validation subsequently ran the suites; this was temporary scheduling, not deferred coverage. |
| Leave cross-OS execution to CI | No OS-specific code changed; local cross-platform evidence is not a readiness requirement. |
| Search the real skill directory rather than its symbolic-link alias | Equivalent source coverage; no architectural change. |
| Leave historical research, already-correct pages, and accurate fallback examples unchanged | Avoids unrelated edits; the current expression documentation describes one grammar. |

No implementation functionality is declared deferred. The only outstanding work reported by the implementer is the unrelated prompt-test maintenance above. Helper type strictness, operand-returning `&&`, and hook-dispatch error policy remain excluded by the spec.
