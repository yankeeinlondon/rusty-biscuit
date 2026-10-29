---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-10-27T19:08:39-07:00
spec: 2026-10-27-shell-site-placeholder-parse-once/spec.md
implemented: false
description: "A **feature** review of `2026-10-27-shell-site-placeholder-parse-once/spec.md`"
feature: 2026-10-27-shell-site-placeholder-parse-once/review-2.md
previous: 2026-10-27-shell-site-placeholder-parse-once/review-1.md
---

# Review 2: shell-site placeholder parsing

The feature is **production ready**. Both previous findings are verified fixed across every required instance. No partial repair, regression, newly discovered sibling, or new finding remains.

Read the specification, plan, previous review, implementation records, and the repair's `### Finding Ledger` in `log.md` before checking the fixes. Reviewed production changes from `94faa2fab` through `ed223084c` and the current working-tree tests and documentation. Verification below comes from implementation inspection and independently running tests, rather than accepting completion flags. The repair changes only Claudine tests. The previous review contains no author rulings; its informational implementation choices remain unchanged.

## Previous Finding Verification

Instance IDs match the previous review and repair ledger. “Verified fixed” includes previously clean instances whose behavior remains correct.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: Parse-count cases never exercise late or guarded placeholders | R1-F1.1–R1-F1.10 | **verified fixed** | Claudine's [static count test](../../lib/src/composition/sequence/preflight/tests/scaling.rs:177) now checks counts, per-occurrence roots, presence proof, and resulting timing. The guard is prepared before the counter snapshot. All children checked below; tests pass with `work-counters`. | Nothing. |
| R1-F1.1: Repeated text | Three occurrences of `a` | **verified fixed** | Three parses, three early occurrences, no presence proof, three approval-time holes. | Nothing. |
| R1-F1.2: Deferred and guarded holes | Real late reads, with and without a presence condition | **verified fixed** | `seq.outputs[0]`, `seq.outputs[1]`, and `a` cost three parses. Under `when: seq.outputs[0]`, only the first is proven present: timings are guarded/deferred/approval. The unguarded control leaves both `seq` reads deferred. | Nothing. |
| R1-F1.3: Failed grammar beside a late read | Redirection failure and fallback root discovery | **verified fixed** | `echo {{ seq.outputs[0] }} > out` costs one parse, fails template grammar, and still reports root `seq` and the condition's presence proof. | Nothing. |
| R1-F1.4: Malformed beside valid | `a +` beside `b` | **verified fixed** | Two parses; both occurrences remain visible. Darkmatter's occurrence tests separately assert the failed parse is retained beside valid syntax. | Nothing. |
| R1-F1.5: Malformed beside late | `seq.outputs[0]` beside `b +` | **verified fixed** | Two parses; the valid read reports `seq` and deferred timing despite the malformed neighbor. | Nothing. |
| R1-F1.6: Plain command | No placeholders | **verified fixed** | Zero parses, no occurrences, successful template with no holes. | Nothing. |
| R1-F1.7: Triple-brace literal | Literal interpolation syntax | **verified fixed** | Zero parses, no expression occurrences, template grammar rejection. | Nothing. |
| R1-F1.8: Data literal token | `{{!data:x}}` | **verified fixed** | Zero parses, no expression occurrences, template grammar rejection. | Nothing. |
| R1-F1.9: Full-preflight controls | Ten and 100 steps with ten holes; four steps with three holes | **verified fixed** | Full-preflight tests pass with exact counts 600, 6,000, and 72, alongside returned-graph assertions. | Nothing. |
| R1-F1.10: Existing condition behavior | Missing `msg`; late `seq.outputs[3]` | **verified fixed** | Existing preflight tests pass: a condition-proven missing value requires a template; a guarded late hole retains root `seq` and avoids the step barrier, while its unguarded control remains deferred. | Nothing. |
| R1-F2: Preserve schema validators and refinements in sequence-preflight tests | R1-F2.1–R1-F2.4 | **verified fixed** | Claudine's [schema-bearing preflight tests](../../lib/src/composition/sequence/preflight/tests.rs:3674) fill templates returned by the graph and assert distinct schema and condition rejection reasons. All children checked below. | Nothing. |
| R1-F2.1: Primary shell schema and condition | Numeric validator, enum refinement, accepted and rejected values | **verified fixed** | Later-task template accepts `2`/`release`, rejects `0` and string `"2"` as `DeclaredType`, and rejects `debug` as `GuardRefinement`. Without the condition, `debug` passes and enum nonmember `nightly` fails as `DeclaredType`. | Nothing. |
| R1-F2.2: Collected stack commands | Setup, teardown, and `on_error` | **verified fixed** | Checked shared wiring: [stack collection](../../lib/src/composition/sequence/preflight/mod.rs:1767) clears the task condition and sends every collected command through the same schema projection in site preparation. The representative collection test passes. | Nothing; the task condition correctly does not apply to stacks. |
| R1-F2.3: Whole-value `set` commands | Shared schema projection and translated positions | **verified fixed** | New container `set` test accepts count `2` and rejects `0` as `DeclaredType`. The extracted command view feeds the same projection. Existing Unicode, wrapper, suffix, and diagnostic-coordinate cases pass. | Nothing. |
| R1-F2.4: Darkmatter projection parity | Text and retained-syntax projection with schema and condition | **verified fixed** | `projected_types_match_with_schema_validators_and_refinements` passes; both entry points share the expression projection implementation. | Nothing. |

New tests use fixture-owned temporary directories and no real providers or terminals. Their library modules are declared, their names remain in Level 1, and both the area recipe and package CI metadata enable `work-counters`. The independent run selected and passed the repaired cases.

## Unblocked Findings

None.

## Blocked Findings

None. No decision requires a human before a repair can proceed.

## Spec Review and Verification

In Claudine, [shell-site preparation](../../lib/src/composition/sequence/preflight/mod.rs:1830) constructs one Darkmatter syntax collection after whole-value shell extraction. Approval validation and identity checks read raw-source occurrences; grammar, schema projection, late-root marking, and presence checks reuse retained syntax. The command view translates byte positions without reparsing. The collection is local to the call and is absent from stored templates and approval records.

In Darkmatter, [retained interpolation syntax](../../../darkmatter/lib/src/markdown/compose/expression/prepared.rs:286) reuses the scanner and parser, retains valid occurrences beside failures, and caches only each occurrence's erased syntax. Existing all-or-error preparation still stops at its first malformed occurrence. Validation ordering, text entry points, and shared type/root/presence analysis remain intact. Inspection found no extra expression parser call in the optimized static path.

| Specification requirement | Verification level and evidence | Assessment |
| --- | --- | --- |
| Acceptance 1: exact measured parse formula | Level 1, full sequence preflight with existing counters enabled; 600, 6,000, and 72 parses | Met. |
| Acceptance 2: repeated, late, guarded, grammar-failure, plain, and literal cases | Level 1, repaired static-analysis matrix plus Darkmatter occurrence/scanner tests | Met; actual roots, proof, and timings are asserted alongside counts. |
| Acceptance 3: errors, approval modes, timing, schema restrictions, identity refusal, missing/falsy values, collected commands, and offsets | Level 1, preflight characterization and behavior tests, new schema-bearing graph tests, and existing isolated CLI tests | Met. Checks preserve winning typed errors and sources, first/later program-hole behavior, derived late values, dotted/bracket/computed identity access, dead-branch validation, and wrapper coordinates. |
| Acceptance 4: fresh evaluation and fixed bytes for exact approvals | Level 1, fixture file edited between fills; executor tests for current-value template fills and fixed-byte execution | Met. File reads return `draft` then `ready`; the separate exact-approval test preserves stamped bytes. |
| Acceptance 5: existing allocation and Criterion samples | Inspected `implementation.md`: revision `ed223084c` plus documented working-tree changes, bench profile and features, one macOS host | Met by recorded samples: 6,000 parses, 25,588,809 allocated bytes, 195,225 allocations, 7.5963 ms median. The record accounts for retained/erased syntax costs and single-sample limitations. Benchmarks were not repeated during review. |
| Acceptance 6: affected-area gates and enabled counter tests | Earlier full-area passes recorded and assessed in review 1; repair log records a later Claudine full pass. This review independently ran focused Nextest tests, Claudine lint, and tier coverage below | No outstanding gate failure. Darkmatter production code is unchanged since the previous review. |
| Acceptance 7: documentation and API guidance | Inspected Claudine preflight docs, Darkmatter template/expression docs and comments, and both skill updates | Met: guidance describes static parsing during one site preparation, unchanged evaluation and approval modes, and no syntax cache across sites. No dated-spec backlink was added to current docs. |

Level 1 is appropriate for every required behavior: passive analysis, returned templates and errors, hermetic file reads, and execution mode selection. No rendering or keyboard-input requirement needs Level 2 or Level 3 evidence. Cross-platform execution evidence remains with CI and does not block this review.

Commands independently run for this review:

| Directory | Command | Result |
| --- | --- | --- |
| `claudine/` | `just test composition::sequence::preflight::tests:: scaling_tests:: bindings::tests:: type_scope_ownership_guard shell_templates` | 623 selected tests passed. |
| `darkmatter/` | `just test prepared_parity interpolation_tests type_scope_ownership_guard` | 38 selected tests passed, including occurrence-counter tests. |
| `claudine/` | `just lint` | Passed, including nine transport guard tests and the lifecycle documentation guard. |
| Repository root | `just check-tier-coverage claudine darkmatter` | Passed; zero stranded tests. |
| Repository root | `git diff --check` | Passed. |

All 661 selected tests passed; no selected test exceeded five seconds. Compilation emitted the previously recorded compact-unwind linker warning. This review did not repeat full area suites or benchmark samples already recorded for the unchanged implementation. No production code was edited or temporarily mutated, and no formatting or commit command was run.

## Input Robustness

The configuration input matrix does not apply. This feature changes an interpolation/template text grammar's preparation path and factors shell-wrapper extraction; it adds no configuration loader or field validator. Malformed expressions, literal syntax, extraction precedence, and source coordinates are verified at their existing grammar and preflight boundaries.

## Recurrence

False. Both classes reported in review 1 are fully repaired. No unchanged defect, repair regression, or missed sibling was found within the specification boundary.
