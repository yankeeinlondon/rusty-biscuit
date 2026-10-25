---
implementation_1: "2026-10-23T00:49:24-07:00"
---

# Schema Enhancements Review Log

## Implementation of Review Findings #1

> **started at:** 2026-10-23T00:49:24-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-pipeline/claudine/features/2026-10-28-schema-enhancements/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- review frontmatter `implemented` confirmed `false` before repairs began

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1   | Commit results do not use the promised shared named type | fixed | all child rows fixed/clean; guard `recent_commits_shared_type` 3/3 pass |
| R1-F1.1 | `ctx.recent_commits` in `context-variables.yaml` references the shared named type, keeping `generated; required` | fixed | `context-variables.yaml:58` now `recent-commits(generated; required)@darkmatter:functions`; guard test asserts reference + constraints |
| R1-F1.2 | `recent_commits(count)` returns `recent-commits` | clean | return still `returns(recent-commits)`; guard asserts the loaded descriptor's return references it |
| R1-F1.3 | one `recent-commits` definition available to both consumers | clean | single definition `functions.yaml:66`; guard asserts it resolves to `string[]` for both consumers |
| R1-F1.4 | function/variable descriptions keep their cross-references | clean | guard asserts both descriptions name each other |
| R1-F1.5 | embedded/on-disk schema copies and fixtures updated together; guard test proves neither consumer owns the element type | fixed | `functions.yaml` added to `SHIPPED` (`embedded.rs`) + table test; reserved import reads no disk, so no fixture copies needed; 4300 darkmatter/dmls + 860 claudine filtered tests pass |
| R1-F1.6 | new import surface: a function entry imported as a property type (`min@darkmatter:functions`) must fail cleanly | clean | orchestrator probe: `md compose` exits 1 with "a `function` type is description-only…"; `recent-commits@darkmatter:functions` composes |
| R1-F2   | Call classification loses preserved text representations | fixed | all child rows fixed/clean; R1-F2.10 decision recorded for author confirmation |
| R1-F2.1 | `fit`/`family`: numberlike→boolean, numberlike→boolish, boolish→number, boolish→numberlike are category 3 | fixed | four cross-family cells now 3; `design_time_call_check::preserved_text` per-cell tests (75 pass) |
| R1-F2.2 | numberlike/boolish → `json` is category 3 | fixed | numberlike/boolish→`json` now 3 (`+4`, `yes` not JSON); per-cell tests |
| R1-F2.3 | `source_kinds`/`arm_fit`: `length(src)` with required boolish source is category 3 | fixed | `source_kinds` yields native+Text for as-sent shapes; `length(src)` boolish → 3, plain boolean still 5 |
| R1-F2.4 | runtime `min("1",5)` and `length("yes")` still compute | clean | runtime tests `min('1',5)`→1, `length('yes')`→3, one handler entry each |
| R1-F2.5 | whole-call recovery `number(src,7)` with boolish source stays category 1 | clean | `number(src,7)` boolish source stays 1 (test) |
| R1-F2.6 | `md compose` of the boolish `min(src, 5)` document composes | fixed | `compose_call_precheck::a_boolish_holding_numeric_text_composes`; shipped `md compose` exits 0 printing `1`; controls compose |
| R1-F2.7 | DMLS: boolish source → `min`/`max`/`abs`/`length` publish category 3 warnings, not errors | fixed | DMLS session test: four `dm.expression.uncertain_call` warnings, no `incompatible_call` |
| R1-F2.8 | DMLS controls (numberlike→`min`, recovered call, literal text→`min`) stay clean | clean | DMLS session test: controls publish no call diagnostics |
| R1-F2.9 | remaining 35-pair matrix cells keep their categories | clean | remaining matrix cells asserted per cell, unchanged except R1-F2.10 |
| R1-F2.10 | numberlike/boolish→`yaml`: review expected 1; tab-led `\t4`/`\tyes` are admitted (trimmed) but not YAML | fixed (decision for author) | now 3; `retained_text_is_not_always_json_or_yaml`; keeping 1 would be false certainty of the same class |
| R1-F2.11 | sibling: numberlike→`literal(true)`, boolish→`literal(1)` | fixed | were 5, now 3; per-cell tests |
| R1-F2.12 | sibling: numberlike/boolish→`alphanumeric` | fixed | were 5, now 3; `number`→`alphanumeric` stays 5 |
| R1-F2.13 | sibling: optional `boolish`, union `[boolish, object]` → `number` | fixed | were 5, now 3; tests |
| R1-F2.14 | sibling: list elements, overload choice, recovery fold, guard narrowing, DMLS severity mapping, runtime exclusions | clean | already combine per-argument answers / inspect actual values; numberlike list → boolean list already 3 |
| R1-F2.15 | composition/editor agreement covers retained text | fixed | `call_check_agreement::composition_and_the_editor_agree_on_retained_text` |
| HR-1    | human inspection of `behavior-ledger.md` closing additions (review `human_review_items`) | nonblocking, author action | not an agent repair; the author records the inspection |

### Work Log

- starting the work on 'R1-F1 shared commit-result type' at 00:49:54
        - defect class: two declarations independently own a data contract that the spec requires to have one named definition (item 9, decision 47)
        - R1-F1.1 fixed: `ctx.recent_commits` in `darkmatter/schemas/partials/context-variables.yaml` is now `recent-commits(generated; required)@darkmatter:functions -> …`; the resolved atom is unchanged (`string[]`, `array_constraints: [Generated, Required]`), so callers receive the same values
        - R1-F1.2 clean: `recent_commits(count)` still `returns(recent-commits)` (local reference)
        - R1-F1.3 clean: `recent-commits` stays defined once, in `functions.yaml`; the catalog loader and the schema resolver read the same `include_str!` text
        - R1-F1.4 clean: both descriptions still cross-reference each other (asserted by the new guard)
        - R1-F1.5 fixed (embedded/on-disk copies): added `functions.yaml` to `schemas/embedded.rs::SHIPPED` (and its table test); the reserved `@darkmatter:` namespace never reads disk, so no on-disk fixture copy (global_catalog matrix, content-policy baselines, DMLS sessions) needed a new file — the reason Phase 4 decision 7 deferred this no longer applies
        - sibling sweep: intersected every `functions.yaml` function name with every `context` key — `recent_commits` is the only pair; `current` shares the `context` type so it inherits the fix; spec has no other "shared named type" requirement (only item 9 / decision 47; ledger rows 66 agree); `recent_commits` entries in `claudine/schemas/{lifecycle,action}.yaml` are verb shapes, not this type — clean
        - docs: `darkmatter/docs/topics/darkmatter-expressions.md` (catalog helper types and the `@darkmatter:functions` import); skills `darkmatter/{compose,library-surfaces,schema}.md` (drifted "still declares its own `string[]`" and stale "five partials" corrected); the generated ctx table in `context-variables.md` shows the resolved `string[]` and is unchanged
        - test added: `darkmatter/lib/tests/l1/recent_commits_shared_type.rs` (3 tests, registered in `tests/l1/main.rs`): authored `ctx.recent_commits` is `Imported{recent-commits, darkmatter:functions}` with generated/required, authored function return is `Imported{recent-commits, this}`, named type and resolved ctx member are `string[]` with list-level generated/required
        - `cargo nextest run -p darkmatter --test l1 -E 'test(recent_commits_shared_type)'` → 3 passed
        - `cargo nextest run -p darkmatter -p darkmatter-cli -p dmls -E 'test(/embedded|catalog|context|shipped|baseline|content_policy|recent_commits|schema|expression|about|function|lsp_session/)'` → 4300 passed, 0 failed
        - `cargo nextest run -p claudine -p claudine-cli -E 'test(/context|schema|lifecycle_schema|action_schema|recent/)'` → 860 passed, 0 failed
        - `just lint` (darkmatter) → clean
        - orchestrator review: inspected the diff; probed the new `@darkmatter:functions` import surface with `md compose` (function entry rejected with a located schema error, helper type composes) — recorded as R1-F1.6
- work completed for 'R1-F1 shared commit-result type' at 01:00:19
- starting the work on 'R1-F2 call classification loses preserved text representations' at 01:00:19
        - defect class: static classification collapses a type with multiple preserved runtime representations (`numberlike`/`boolish`, `as_sent`) to one primitive family, producing false rejection (category 5) and false certainty (category 1)
        - R1-F2.1 fixed: `fit` in `darkmatter/lib/src/markdown/compose/expression/call_check.rs` now splits an `as_sent` source into its native scalar plus retained text (new source-only `Family::NumberText`/`BooleanText`, `retained_text_fit`), combined with `Fit::all`; boolish→number/numberlike and numberlike→boolean/boolish are category 3
        - R1-F2.2 fixed: retained text → `json` is category 3 (`+4`, `yes` invalid JSON)
        - R1-F2.3 fixed: `source_kinds` returns `[Number, Text]` / `[Boolean, Text]` for `as_sent` shapes; `length(boolish)` is category 3, `length(boolean)` still 5
        - R1-F2.4 clean: runtime unchanged; `min('1',5)`→1, `length('yes')`→3, also through a lookup (`src: "1"`, `word: "yes"`), one handler entry per call
        - R1-F2.5 clean: `number(src, 7)` with boolish stays category 1
        - R1-F2.6 fixed: the boolish document composes to `1` (library test and shipped `md compose`, exit 0); number/numberlike/literal controls compose; plain `boolean` still rejected; native `true` in a boolish fails at runtime, not design time
        - R1-F2.7 fixed: DMLS session publishes four `dm.expression.uncertain_call` warnings (min, max, abs, length), no `incompatible_call`
        - R1-F2.8 clean: numberlike→min, `number(flag, 7)`, `min("1", 5)` publish no call diagnostics
        - R1-F2.9 clean except two cells: all non-bold matrix cells kept, EXCEPT numberlike→yaml and boolish→yaml now category 3 (review table said 1): `"\t4"` and `"\tyes"` are admitted (admission trims) but are invalid YAML, proven by `retained_text_is_not_always_json_or_yaml`; author to confirm (new row R1-F2.10)
        - sibling sweep: `literal_family`/literal-target arm (numberlike→`literal(true)`, boolish→`literal(1)` were 5 → now 3, row R1-F2.11); scalar-refusing text `alphanumeric` (numberlike/boolish were 5 → 3, row R1-F2.12); optional boolish and union `[boolish, object]` → number (were 5 → 3, row R1-F2.13); array elements (`numberlike[]`→`boolean[]` 3, →`number[]` 1) already handled by recursion, clean; union targets now evaluated per representation (boolish→`number | string` 1); overload selection/recovery/`CallCheck::category` consume per-argument categories, clean; scope narrowing (`scope/shape.rs::families`) keeps every family for numberlike/boolish (conservative), clean; DMLS consumers (`diagnostics/call_check.rs::severity`, `providers/frontmatter.rs::is_boolish` completion) only map categories / suggest values, clean; runtime `bind.rs` exclusions use the runtime value kind, clean; only numberlike/boolish carry `as_sent` in the coercion `Shape`
        - tests added: `darkmatter/lib/tests/l1/design_time_call_check.rs::preserved_text` (35-cell macro matrix + 11 sibling cells + 6 tests); `compose_call_precheck.rs::conversions::a_boolish_holding_numeric_text_composes`; `dmls/tests/l1/expression_call_diagnostics.rs::preserved_text` (2 session tests); `dmls/tests/l1/call_check_agreement.rs::composition_and_the_editor_agree_on_retained_text` (separate corpus/test; folding it into the original corpus pushed that test from 2.6 s to 4.1 s)
        - new tests fail against the HEAD classifier (8 lib failures before fail-fast, both DMLS tests) and pass with the fix
        - docs: `darkmatter/docs/inline/call-check.md` (new rule with table and example, rejected-table row); skill `.claude/skills/darkmatter/expressions.md` (as_sent invariant)
        - `cargo nextest run -p darkmatter --features effects-instrumentation --test l1 -E 'test(/design_time_call_check/)'` → 75 passed
        - `cargo nextest run -p darkmatter --features effects-instrumentation -p darkmatter-cli --features darkmatter-cli/test-fixtures -p dmls --features dmls/effects-instrumentation -E 'test(/call_check|call_diag|incompatible|uncertain|precheck|agreement|binder|binding|coerc|numberlike|boolish|preserved_text|retained_text/)'` → 552 passed, 0 failed
        - same packages, `-E 'test(/expression|function|compose|diagnostic|catalog|schema/)'` → 6351 passed, 0 failed
        - `cargo nextest run -p claudine -p claudine-cli -E 'test(/call_check|rejected_call|incompatible/)'` → 3 passed, 0 failed
        - `just lint` (darkmatter) → clean
        - orchestrator review: inspected `call_check.rs` diff (as-sent shapes scored as native and retained text, combined with `Fit::all`); accepted the yaml-cell departure as R1-F2.10 for author confirmation
- work completed for 'R1-F2 call classification loses preserved text representations' at 01:16:47
- starting phase gates (darkmatter and claudine `just test`, `just lint`, `just check-tier-coverage`) at 01:16:47
        - darkmatter `just test`: 11703 passed, 0 failed, 10 skipped (63.9 s)
        - darkmatter `just lint`: clean; `just check-tier-coverage darkmatter`: 0 stranded
        - claudine `just test`: 10692 passed, 0 failed, 9 skipped (306.7 s)
        - claudine `just lint`: exit 0; `just check-tier-coverage claudine`: 0 stranded
        - cross-OS check skipped: the changes are a schema YAML line, an embedded-document list entry, and classifier logic, with no path, process, or OS-specific code
        - recorded the R1-F1 departure from Phase 4 decision 7 and the R1-F2.10 decision in `implementation-log.md` under "Review 1 repairs"
- phase gates completed at 01:26:47

### Completion Report

- **outcome: complete**: elapsed about 47 minutes (00:49:24 → 01:26)
- parent findings: total 2 = fixed 2 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
        - R1-F1 fixed: `ctx.recent_commits` and `recent_commits(count)` share the single `recent-commits` definition; new guard `darkmatter/lib/tests/l1/recent_commits_shared_type.rs`
        - R1-F2 fixed: `numberlike`/`boolish` are classified as native scalar plus retained text; the 35-cell matrix, siblings (literal, alphanumeric, optional and union sources), composition, DMLS, and agreement tests were added
- open finding/child IDs: none
- for the author to confirm (decisions, not open defects):
        - R1-F2.10: `numberlike`/`boolish` → `yaml` is category 3, where the review expected 1
        - R1-F1.6: the `@darkmatter:functions` reserved import now exposes the catalog's helper types to user schemas; a function entry imported as a type is rejected with a located schema error
        - HR-1: the review's human inspection of the `behavior-ledger.md` additions is still the author's to record
- changed files:
        - R1-F1: `darkmatter/schemas/partials/context-variables.yaml`, `darkmatter/lib/src/markdown/schemas/embedded.rs`, `darkmatter/lib/tests/l1/recent_commits_shared_type.rs` (new), `darkmatter/lib/tests/l1/main.rs`, `darkmatter/docs/topics/darkmatter-expressions.md`, `.claude/skills/darkmatter/{compose,library-surfaces,schema}.md`
        - R1-F2: `darkmatter/lib/src/markdown/compose/expression/call_check.rs`, `darkmatter/lib/tests/l1/design_time_call_check.rs`, `darkmatter/lib/tests/l1/compose_call_precheck.rs`, `darkmatter/dmls/tests/l1/expression_call_diagnostics.rs`, `darkmatter/dmls/tests/l1/call_check_agreement.rs`, `darkmatter/docs/inline/call-check.md`, `.claude/skills/darkmatter/expressions.md`
        - records: `implementation-log.md`, `log.md`, `review-1.md` (frontmatter)
- final verification:
        - darkmatter `just test` 11703/11703 passed and `just lint` clean: R1-F1, R1-F2
        - claudine `just test` 10692/10692 passed and `just lint` clean: R1-F1 (Claudine embeds the ctx schema), R1-F2 (call-check rejection)
        - `just check-tier-coverage` for darkmatter and claudine: 0 stranded
