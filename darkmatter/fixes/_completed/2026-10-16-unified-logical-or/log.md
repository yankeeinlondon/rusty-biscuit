---
fix: 2026-10-16-unified-logical-or
implementation_1: "2026-10-30T21:07:45-07:00"
---

# Review-Fix Log: Unified Logical OR

## Implementation of Review Findings #1

> **started at:** 2026-10-30T21:07:45-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-schema-narrowing-and-extraction/darkmatter/fixes/2026-10-16-unified-logical-or/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review's `implemented` flag was already `false` at the start of this cycle; left `false` until completion

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1   | Shipped-condition corpus does not check passive call-check compatibility | fixed | all six child rows closed; gate `shipped_conditions_keep_their_call_check_verdict` passes over 224 shipped conditions (22 with `\|\|`), no verdict differs; negative control proves the comparison fails on a changed verdict |
| R1-F1.1 | Darkmatter corpus: bare lifecycle `when`/`while`/`until` conditions have their passive call-check verdict compared with the pre-unification baseline | fixed | `ConditionKey` surface: 80 compared, 5 with `\|\|`, under `lifecycle_declared_types(effective, EventRegistry::md())`; gate asserts compared == present and ≥1 `\|\|` condition |
| R1-F1.2 | Darkmatter corpus: directive `when` and shell-ternary condition surfaces included in the comparison | fixed | `WhenDirective` (every directive's `when=`: `::block`, `::file`, `::code`) 144 compared, 17 with `\|\|`; `ShellTernaryCondition` and `SchemaExpression` included in `CONDITION_SURFACES` (none shipped today; extraction pinned by existing fixtures); scope `DeclaredTypes::for_schema(effective)` as the compose precheck builds it |
| R1-F1.3 | Claudine CLI shipped-prompt contract: frontmatter (lifecycle / interpolated) conditions call-checked under effective declarations and host scope, or covered once by the shared Darkmatter corpus | fixed (covered once by the Darkmatter corpus) | temporary probe replicating Claudine's extraction: all 80 frontmatter `when`/`while`/`until` values in `prompts/*.md` are extracted by the Darkmatter corpus for the same files, 0 missing (probe removed); Claudine test doc now points to the gate. Decision: Claudine host globals are untyped in the comparison (not passively constructible from Darkmatter); both trees share the scope, recorded in `human_review_items` |
| R1-F1.4 | Claudine CLI shipped-prompt contract: body block conditions call-checked, or covered once by the shared Darkmatter corpus | fixed (covered once by the Darkmatter corpus) | same probe: all 130 `::block when="…"` lines found as `WhenDirective`, 0 missing; no Claudine-only condition surface exists |
| R1-F1.5 | `check_expression_calls` keeps the original/edited distinction (negative control: `length(true) > 0 \|\| …` rejected) | clean (now pinned) | `call_check_distinguishes_a_rejected_condition_call`: original has no rejected call, edited rejects exactly `length` at 0..12; `call_verdict_comparison_reports_a_changed_verdict` proves the LogicalOr-vs-`or(…)` comparison reports `length(items \|\| []) > 0` |
| R1-F1.6 | Implementation log's acceptance mapping for criterion 3 updated | fixed | `implementation-log.md` AC3 entry now says the parse corpora are syntax-only and cites the call-check gate and its three controls |

### Progress

- starting the work on 'R1-F1: shipped-condition passive call-check compatibility' at 21:13:40
        - defect class: a compatibility corpus test verifies that shipped expressions parse but never asks the later passive decision (the call check) whether its verdict on each condition is unchanged
        - orchestrator sweep before delegation: two parse-only corpora exist (Darkmatter `dasherized_identifier_corpus.rs` over `prompts/`, `.claude/commands/`, `darkmatter/prompts/`, `claudine/prompts/`; Claudine CLI `shipped_prompt_contract.rs` over `prompts/`); the Darkmatter corpus walks a superset of the Claudine one, so the comparison is placed there once, as the review recommends
        - delegated to one foreground `rust-developer` subagent (serial, single finding)
        - discovered: condition surfaces (formerly parsed in condition mode) in the Darkmatter corpus are `ConditionKey`, `WhenDirective` (every directive's `when=`, so `::block`, `::file`, `::code`), `ShellTernaryCondition`, and `SchemaExpression`; the shipped corpus holds only the first two (80 `ConditionKey`, 144 `WhenDirective`). `::map`/`::filter` `when=` gates were also condition-parsed but no shipped artifact has one; the one shipped `::map` has only a `where=` callback, which was never condition-parsed
        - discovered: a discriminating input exists: `length(items || []) > 0` with `items` untyped rejects `length` when `||` is the boolean `or(…)` call and does not as `LogicalOr`
        - completed: `darkmatter/lib/tests/prompts/dasherized_identifier_corpus.rs` gained gate `shipped_conditions_keep_their_call_check_verdict`: each condition is parsed with `parse_spanned`, projected test-only back to `or(left, right)` (span = the `LogicalOr` span), and both trees run through `check_expression_calls` in the same scope; the rejected-call sets (`CallCheck::rejection_diagnostic().is_some()`, the predicate of the compose precheck and `reject_incompatible_calls`, keyed by function + span, synthesized `or` calls excluded) must match. Scope: `DeclaredTypes::for_schema(effective)` per document; lifecycle keys use `lifecycle_declared_types(effective, EventRegistry::md())` (Claudine host globals and enclosing guards are not passively available; documented on `condition_declarations`)
        - completed: controls `call_check_distinguishes_a_rejected_condition_call` (review pair), `call_verdict_comparison_reports_a_changed_verdict` (`length(items || [])`), `projection_marks_only_synthesized_or_calls`; the gate asserts every present condition parsed and was compared and that `ConditionKey` and `WhenDirective` each have `||` conditions (5 and 17 of 224 compared); no verdict differs; module docs updated
        - completed: Claudine contract coverage verified with a temporary probe (removed): its 80 frontmatter `when`/`while`/`until` values and 130 `::block when="…"` lines over `prompts/*.md` are all extracted by the Darkmatter corpus for the same files (0 missing); one doc line on the Claudine test points to the Darkmatter gate
        - verification: `cargo nextest run -p darkmatter --features prompt-tests --test prompts` 14/14 passed; `cargo clippy -p darkmatter --features prompt-tests --test prompts -- -D warnings` clean; `just lint` (darkmatter) clean; `just test-prompts shipped_prompts_have_parseable_schemas_and_expressions` (claudine) 1 passed
        - orchestrator review: inspected the combined diff; the projection's `match` is exhaustive over `SpannedExprKind` (a new node kind fails to compile rather than escaping the projection); synthesized `or` calls are excluded by span only, so an authored `or(…)` is still judged
        - orchestrator: updated the AC3 acceptance mapping in `implementation-log.md` (R1-F1.6)
- work completed for 'R1-F1: shipped-condition passive call-check compatibility' at 21:19:48

### Completion Report

- **outcome:** complete
- **elapsed:** about 12 minutes (21:07:45 → 21:20)
- **parent findings:** total 1 = fixed 1 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
        - child rows: 6 (R1-F1.1, R1-F1.2, R1-F1.6 fixed; R1-F1.3, R1-F1.4 fixed by single coverage in the Darkmatter corpus; R1-F1.5 clean, now pinned by a test)
- **open findings:** none
- **verified repair:** Darkmatter's opt-in shipped-artifact corpus gained the gate `shipped_conditions_keep_their_call_check_verdict`. It call-checks every shipped condition twice in the same scope: once as parsed, and once with each `LogicalOr` projected back to the former boolean `or(…)` call. The two sets of rejected calls must be equal. Controls prove that the checker and the comparison both detect a changed verdict.
- **decision recorded for the author:** the comparison types Claudine host globals as unknown. Darkmatter cannot passively build Claudine's host registry, and the corpus does not know each condition's event. Enclosing-guard narrowing is not applied either. Both trees are judged in the same scope, so a verdict change could be hidden only where that missing scope alone decides it. The alternative was a Claudine-side comparison, which would add a second comparison over a subset of the same files.
- **cross-OS:** not run; the change is test-only code with no path or OS-specific logic (the corpus walk itself is unchanged)
- **changed files:**
        - `darkmatter/lib/tests/prompts/dasherized_identifier_corpus.rs` (R1-F1.1, R1-F1.2, R1-F1.5)
        - `claudine/cli/tests/prompts/shipped_prompt_contract.rs` (doc comment only; R1-F1.3, R1-F1.4)
        - `darkmatter/fixes/2026-10-16-unified-logical-or/implementation-log.md` (R1-F1.6)
- **final verification:**
        - `cargo nextest run -p darkmatter --features prompt-tests --test prompts`: 14/14 passed (R1-F1.1, R1-F1.2, R1-F1.5)
        - `cargo clippy -p darkmatter --features prompt-tests --test prompts -- -D warnings`: clean (subagent run)
        - `just test-prompts shipped_prompts_have_parseable_schemas_and_expressions` (claudine): 1 passed (R1-F1.3, R1-F1.4)
        - `just test` (darkmatter): 12884 passed, 10 skipped
        - `just lint` (darkmatter): exit 0, no warnings
        - `just test` (claudine): 11810 passed, 15 skipped
        - `just lint` (claudine): exit 0; only the pre-existing macOS linker `__eh_frame` warning
