---
$schema: feature-review.yaml
ready: false
findings:
    - "R1-F1 (high): Parse-count cases never exercise late or guarded placeholders"
    - "R1-F2 (high): Preserve schema validators and refinements in sequence-preflight tests"
human_review: true
human_review_items:
    - "For later confirmation or reversal: keep expression offsets on individual occurrences so the existing public parse-error type stays unchanged; recommendation is to accept."
    - "For later confirmation or reversal: borrow occurrence views with translated source positions instead of cloning syntax records; recommendation is to accept."
    - "For later confirmation or reversal: let the source-policy tests recognize the new shared type-scope and parsed-expression helpers; recommendation is to accept."
    - "For later confirmation or reversal: measure isolated static parsing with a test helper, alongside the full-preflight count; recommendation is to keep this structure and repair its missing cases."
    - "For later confirmation or reversal: reject conditional shell-value templates inside sequence preparation while preserving the existing error; recommendation is to accept."
    - "For later confirmation or reversal: keep local verification scoped to the two affected areas and leave additional operating-system evidence to CI; recommendation is to accept."
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-10-27T18:41:30-07:00
spec: 2026-10-27-shell-site-placeholder-parse-once/spec.md
implemented: true
implemented_by: claude/opus
log: /Volumes/coding/wt/rusty-biscuit/feat-work-reduction/claudine/features/2026-10-27-shell-site-placeholder-parse-once/log.md
description: "A **feature** review of `2026-10-27-shell-site-placeholder-parse-once/spec.md`"
feature: 2026-10-27-shell-site-placeholder-parse-once/review-1.md
next: 2026-10-27-shell-site-placeholder-parse-once/review-2.md
---

# Review 1: shell-site placeholder parsing

The feature is **not production ready** because two explicit acceptance requirements lack durable tests of the promised states. No incorrect runtime result was reproduced. Both repairs can stay within the specification and require only focused Level 1 test changes.

Reviewed the implementation from `94faa2fab` through `ed223084c`, the current documentation changes, the specification, plan, and implementation records. Two independent agents read the decisions and scope before the code review. There is no previous review of this feature.

## Implementation decisions (informational)

None of these choices meets the test for stopping for a human. The recommendations remain within the specification, and none blocks a repair.

1. **Store offsets on occurrences.** In Darkmatter, each retained occurrence keeps its expression offset independently of a parse failure. This avoids changing the existing public error structure. Accept: it preserves the required coordinates and compatibility.
2. **Borrow translated views.** Darkmatter's occurrence view translates positions and provides lookup by placeholder span without cloning syntax or failures. Accept: it implements the required raw-wrapper and inner-command views. The specification expressly leaves helper names to implementation.
3. **Update source-policy checks for the shared helpers.** The tests now recognize the factored schema-free type scope and parsed presence-check helper. Accept: inspection and the passing ownership-guard tests show the original policy remains enforced.
4. **Measure static analysis with a separate helper.** The helper repeats the production analysis sequence without evaluation; the full-preflight counter separately checks the measured workload. Accept this reversible structure, which the plan authorizes. Finding R1-F1 repairs its inputs and assertions; extracting a new production abstraction is unnecessary.
5. **Place conditional rejection in sequence preparation.** Claudine checks the extracted shell value's conditional flag before constructing its template. Accept: this preserves the conditional grammar error, extraction precedence, and the exact-byte route without widening the template API.
6. **Limit local validation to the affected areas.** The implementation ran the Claudine and Darkmatter gates and one host's required benchmark samples. It omitted additional dependent-package suites because the shared API changes are additive and the changed consumer is Claudine; additional operating-system evidence remains with CI. Accept this scope for the feature.

No required functionality is recorded as deferred. Separate frontmatter scans, removed-read checks, evaluation parsing, and discovery routes outside this preparation method are explicit exclusions. Documentation postponed during earlier phases was completed. The recorded slow provider-hint test is unchanged; the focused review runs did not reproduce a slow test, and this review does not infer a new defect from that historical observation alone.

## Findings

### R1-F1 (high): Parse-count cases never exercise late or guarded placeholders

**Authority — spec, Acceptance 2:** “verify one static parse per occurrence for repeated text, late and guarded holes, and a command that fails template grammar.” The named late/guarded cases in Claudine's [static count test](../../lib/src/composition/sequence/preflight/tests/scaling.rs:145) use `outputs.x`, but [the sequence's declared late roots](../../lib/src/composition/sequence/preflight/mod.rs:86) are `seq`, `loop`, and `group`. Its [measurement helper](../../lib/src/composition/sequence/preflight/tests/scaling.rs:111) always constructs an unguarded type scope. Consequently, none of the eight rows exercises a late root or a condition that proves a placeholder present.

**Defect class:** a test counts work without proving that its input reaches the analysis state it claims to cover.

A temporary in-process probe used the same eight strings with Darkmatter's public retained-syntax, late-root, grammar, and presence APIs. All rows returned no late roots and no presence proof. The positive control `seq.outputs[0]`, under that same condition, returned the late root `seq` and a presence proof. The probe passed and was removed; it confirmed the missing premises rather than a production failure.

The complete sweep of the count matrix and its nearby controls is below. Child IDs identify repair-ledger rows; clean rows need no change.

| Instance | Site / shape tested | Observed result | Expected result / repair |
| --- | --- | --- | --- |
| R1-F1.1 | Count row: repeated `a` three times | Three parses; no late roots or presence proof | Clean: three separate occurrences are counted. |
| R1-F1.2 | Count row: “deferred and guarded holes,” `outputs.x` plus `a` | Two parses; both reads are early and unguarded | Use an actual declared late root and a scope containing a presence condition; assert both analysis outcomes as well as the count. |
| R1-F1.3 | Count row: failed grammar beside `outputs.x` | One parse and rejected redirection grammar; no late root | Keep the grammar failure, replace the read with a real late read, and assert fallback analysis still reports its root. |
| R1-F1.4 | Count row: malformed `a +` beside valid `b` | Two parses; one retained failure | Clean for the malformed-plus-valid count requirement. |
| R1-F1.5 | Count row: “mixed malformed and late,” `outputs.x` beside malformed `b +` | Two parses; no late root | Use a real late read and assert the malformed neighbor does not hide it. |
| R1-F1.6 | Count row: plain command | Zero parses | Clean. |
| R1-F1.7 | Count row: triple-brace literal | Zero parses; template grammar rejects the literal | Clean. |
| R1-F1.8 | Count row: data literal token | Zero parses; template grammar rejects the token | Clean. |
| R1-F1.9 | Full-preflight count controls: 10 and 100 steps, ten present typed holes; four steps with three holes | 600, 6,000, and 72 parses; expected graph assertions pass | Clean for Acceptance 1; these early, present values do not substitute for the missing late/guarded count cases. |
| R1-F1.10 | Existing sequence-preflight condition tests: missing `msg` and late `seq.outputs[3]` | Correct template mode, guarded timing, and reported roots | Clean behavior coverage; no isolated parse-count assertion is present. |

**Smallest repair:** let the existing measurement helper accept a prepared type scope or an optional condition, prepare that condition before starting the counter, and correct the three mislabeled late rows. Retain a genuine deferred case and a genuine condition-proven case, with assertions that they reach those states. Keep the current counter and existing framework. No production refactor or new shell/terminal test is needed.

### R1-F2 (high): Preserve schema validators and refinements in sequence-preflight tests

**Authority — spec, Acceptance 3:** “Behavior coverage through sequence preflight preserves” late and condition-proven placeholders, including “unchanged timing, validators, refinements, and reported roots.” The new Darkmatter [schema projection test](../../../darkmatter/lib/src/markdown/compose/shell_expansion/template/tests.rs:557) supplies its own schema and scope directly. It does not exercise Claudine's [selection of the owning schema and task condition](../../lib/src/composition/sequence/preflight/mod.rs:1900), or inspect their effect in the returned preflight graph. The sequence tests assert timing and roots, but the only direct hole-type assertion there checks a schema-free broad string. The CLI's ordinary string fixture likewise cannot distinguish its intended schema from the broad fallback.

**Defect class:** helper-level type parity leaves the host's schema-and-condition wiring without the required result-level regression test.

The realistic input is a sequence command `tool {{ count }} {{ mode }}`, with `count: number(min(1))`, `mode: enum(release, debug)`, present values `2` and `release`, and task condition `mode == 'release'`. A temporary probe through sequence preflight confirmed the current graph retains a numeric validator that rejects `0` and the string `"2"`, and a refinement that accepts `release` and rejects `debug`. The probe passed and was removed. That evidence establishes a small safe repair; the implementation needs a lasting test of this contract.

| Instance | Site / shape checked | Observed result | Expected result / repair |
| --- | --- | --- | --- |
| R1-F2.1 | Claudine primary shell site with the numeric schema and enum condition above | Temporary preflight probe preserves both restrictions; committed tests do not assert them | Retain equivalent assertions on the returned template, or fill it with valid and invalid values and assert the corresponding outcomes. |
| R1-F2.2 | Collected setup, teardown, and `on_error` commands | All call the same preparation/projection body; their scope explicitly carries no task condition. Existing collection fixture resolves undeclared `word`. | Clean shared wiring by invariant; do not impose the task condition on stacks or create a separate matrix. The representative schema test can cover the shared projection. |
| R1-F2.3 | Whole-value `set` command | Uses that same projection with translated inner-command spans; existing Unicode/wrapper tests verify occurrence mapping | Clean shared wiring by invariant; preserve the existing coordinate tests. |
| R1-F2.4 | Darkmatter standalone text-versus-prepared projection | Passing parity test includes numeric constraints and enum declarations | Clean shared implementation coverage; retain it alongside the missing host-boundary test. |

**Smallest repair:** extend the existing sequence-preflight behavior tests with one hermetic schema-bearing command and a condition that actually narrows an enum or union. Assert the retained validator and refinement through the returned graph or template-fill result, including one accepted value and one rejected value for each restriction. This requires no production change and no process-spawn matrix.

Both findings are unblocked. There are no low-priority findings and no decision requiring an author ruling.

## Spec review and verification

The retained collection owns syntax and located failures only. It is constructed once after shell-value extraction and dropped on every return from site preparation. Cached erasure is per occurrence; source-position lookup keeps repeated text distinct. No collection, schema, condition fact, value, or verdict is added to an approval record or stored template.

The shared validator preserves normal-diagnostic ordering followed by late-namespace diagnostics. A malformed occurrence suppresses approval diagnostics while valid target-identity reads remain visible. The new extraction helper preserves the old wrapper and suffix handling. Grammar, root traversal, type projection, and presence lookup delegate to shared analysis bodies. Inspection found no additional text reparse inside the changed static path.

| Requirement | Strongest relevant verification | Result |
| --- | --- | --- |
| Acceptance 1: measured parse formula | Level 1, existing counter through full preflight | Exact counts pass with `work-counters` enabled. |
| Acceptance 2: repeated, late, guarded, failed-grammar, and literal cases | Level 1, isolated count helper and retained-syntax tests | Repeated/malformed/literal controls pass; late and guarded cases need R1-F1. |
| Acceptance 3: winning typed errors, identity checks, missing/falsy values, first/later approval, offsets, and collected commands | Level 1, sequence-preflight and hermetic CLI cases | Passing characterization and existing behavior tests; schema validator/refinement wiring needs R1-F2. |
| Acceptance 4: file-read freshness and exact bytes versus fresh template fills | Level 1, fixture-file edit between fills and existing executor tests | Passed. No value caching was observed. |
| Acceptance 5: existing A6 sample | Implementation record inspected; benchmark not repeated during review | Records revision/build configuration, 6,000 parses, 25,588,809 allocated bytes, and 7.5963 ms median. Discloses single-run limitations and conversion costs. No speedup threshold is required. |
| Acceptance 6: area gates | Implementation log records full `just test` and `just lint` passes in both areas | Review independently reran the focused tests below; it did not repeat full gates. |
| Acceptance 7: current documentation | Reviewed Claudine/Darkmatter topic pages, API comments, and both skill updates | Describes preparation-local syntax reuse and unchanged evaluation/approval behavior; no dated-spec backlink was introduced. |

Review commands used Nextest through the area recipes:

| Directory | Command | Result |
| --- | --- | --- |
| `claudine/` | `just test composition::sequence::preflight::tests::` | 230 passed. |
| `claudine/` | `just test scaling_tests::` | 14 passed, including the required enabled counter tests. |
| `claudine/` | `just test bindings::tests:: type_scope_ownership_guard shell_templates` | 376 passed. |
| `darkmatter/` | `just test prepared_parity interpolation_tests type_scope_ownership_guard` | 38 passed. |
| Repository root | `just check-tier-coverage claudine darkmatter` | Passed; zero stranded tests. |

All 658 existing selected tests passed; none exceeded five seconds. Two additional temporary review probes passed and were removed, restoring the source files exactly. Compilation emitted the already recorded compact-unwind linker warning. The initial attempt to pass compound filter expressions through the recipes failed at shell argument parsing before tests ran; the positional filters above replaced it.

Changed unit-test modules are declared by their library parents, remain in Level 1, and are enabled by the area's and CI's `work-counters` features. These requirements concern in-process analysis and hermetic execution, so Level 1 is appropriate. No terminal rendering, keyboard input, or other requirement calls for Level 2 or Level 3 evidence. Additional operating-system proof remains with CI and does not affect this review's readiness decision.

## Input robustness

The input robustness matrix does not apply: this change reads an interpolation/template text grammar and factors existing shell-value extraction. It adds no configuration loader, field deserializer, persisted format, or default. Malformed-expression retention, malformed-wrapper precedence, literal syntax, and source coordinates are covered by the grammar and preflight tests above.

## Recurrence

False. This is the first review of this feature; there are no carried findings or previous repairs to assess.
