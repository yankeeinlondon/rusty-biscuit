---
$schema: feature-review.yaml
ready: false
findings:
  - "R2-F1 (high): Scalar continuation completion still loses composition context"
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
reviewed-by: codex/gpt-6.1-sol
recurrence: true
created: "2026-11-22T19:40:54+00:00"
spec: 2026-11-22-object-composition/spec.md
implemented: true
description: "A **feature** review of `2026-11-22-object-composition/spec.md`"
feature: 2026-11-22-object-composition/review-2.md
previous: 2026-11-22-object-composition/review-1.md
log: /home/build/wt/rusty-biscuit/fix-biscuit-terminal/simplified-schema/features/2026-11-22-object-composition/log.md
implemented-by: claude/opus
next: 2026-11-22-object-composition/review-3.md
---

# Object composition review, iteration 2

The feature is **not production ready**. All five findings from the first review are verified fixed in their required instances. One newly found sibling of the first review's editor-completion defect remains: scalar continuation lines still lose composition context in several ordinary authoring presentations. No finding requires a human decision.

## Previous Finding Verification

The repair's `log.md` Finding Ledger was read before reviewing new work. Its claims were checked against source, the registered tests, and the runs below. There are no earlier reviews or author rulings in this directory beyond review-1.md; its post-repair human-review item is a recommendation, not a ruling.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: Completion disappears in continued flow-sequence operands | R1-F1.1–R1-F1.11, plus ledger R1-F1.S1–S5 | **verified fixed** | Inspected and ran `composition_extends_flow_sequence_continued_onto_later_lines`, frontmatter/union-arm controls, scalar/block/same-line controls, eligibility and receiver tests, wrapped-quote controls, and `operands_on_continuation_lines_complete_navigate_and_report`. All pass in the 92-test DMLS run. Independent provider probes confirm continued flow items offer `base` and quoted flow-reference continuations offer `this`. | No original instance remains. R2-F1 is a newly found sibling, not an unchanged original instance or a regression demonstrated against the repair. |
| R1-F2: Multiline quoted object definitions fail source projection | R1-F2.1–R1-F2.8, plus ledger R1-F2.S1–S3 | **verified fixed** | Both quote styles, ordinary/spread objects, one-line/folded controls, and the wrapped `base@this` operand pass through public loading/lowering. Inspected the 15-presentation location test, LF/CRLF coverage, suggestion-span tests, and DMLS grammar-range test; they ran in the passing simplified-schema and DMLS suites. The shared locator now consumes whole quoted scalars and multiline flow values. | None. Editor completion on these presentations is assessed separately under R2-F1. |
| R1-F3: Deleted sequence type source still prevents consumers from compiling | R1-F3.1–R1-F3.8 | **verified fixed** | `sequence-types.yaml` exists, contains the named declarations, and is included by the production bundle and restored corpus fixtures. `cargo check --locked -p claudine -p claudine-cli -p darkmatter-cli --tests` succeeds, compiling all affected production/test includes, including the eleven CLI bundle sites. Frozen benchmark consumers retain their existing fixture. Runtime bundle checks are recorded below. | None. |
| R1-F4: Shipped Claudine triggers no longer satisfy their consumers | R1-F4.1–R1-F4.7 | **verified fixed** | The restored sequence/type split uses the current trigger envelope; the incomplete shipped lifecycle duplicate is absent while the registration draft remains. Simplified-schema's shipped corpus passes in both feature builds; all 12 Darkmatter shipped-corpus tests pass; DMLS discovery, including removed sequence-key diagnostics, passes. The embedded composition runtime/prepared controls also pass. | None. The spec explicitly leaves host registration to subsequent work. |
| R1-F5: Envelope admission reserves `$extends` inside type tables | R1-F5.1–R1-F5.8, plus ledger R1-F5.S1–S5 | **verified fixed** | The public factory test loads primitive, object, union, and alias declarations named `$extends`, including tagged, informal, and yaml-schema envelopes. Object-position reservation controls pass. Inspected `SchemaOptions::judge`, trigger matching, table parsing, and DMLS table classification/location. Added table-key refusal, declaration narrowing, registry, and DMLS diagnostic/hover controls pass in the runs below. | None. |

## Unblocked Findings

### R2-F1 (high): Scalar continuation completion still loses composition context

**Authority — spec:** under Editor support, “The Darkmatter language server should offer the existing named-type name and import completions after `...`, in a scalar `$extends` value, and in each flow or block sequence item,” and “Completion should work while the operand is incomplete.” Ordinary multiline scalar presentations still return no completion, or offer built-in primitive types that cannot be composition targets. This is a realistic authoring case: users wrap an object definition or a reference, or type a spread before closing its surrounding quote.

**Class:** editor operand discovery loses the owning composition context on scalar continuation lines; fallback routing can then classify an operand as an ordinary type definition.

In Darkmatter's language-server package, [composition_value_start](../../../darkmatter/dmls/src/providers/frontmatter.rs:207) finds continued flow lists but otherwise reads only the cursor line. Its scalar `$extends` route consequently never reaches the operand cursor for a wrapped quoted reference or a block scalar. The [continued_scalar_cursor](../../../darkmatter/dmls/src/providers/frontmatter.rs:461) fallback supports a wrapped quoted definition only after a closing quote exists and excludes literal/folded block scalars. A parsed standalone model sometimes supplies a fallback region, explaining why the same scalar behaves differently in standalone YAML and Markdown frontmatter. That fallback is insufficient for `$extends`: it offers ordinary type keywords instead of enforcing composition eligibility.

For example, request completion immediately after `b`:

```markdown
---
$schema:
  base: '{ id: string }'
  event: >-
    { at: string,
      ...b }
---
```

The server returns no items; it should offer `base`. A temporary test using the existing LSP session fixture reproduced this through `textDocument/completion` and failed the assertion `[] == ["base"]` in 0.064 seconds. The YAML and inline object are valid; this case needs no malformed-input assumption.

A second example is `$extends: 'base@` continued on the next line with `th'`. The completed reference is already supported by the library (the repaired R1-F2.7 case), but requesting completion after `th` returns no `this` suggestion. A flow-list item with that same wrapped reference works, while a block-list item does not.

#### Complete sibling sweep

All rows use the existing `composition_tests::library` fixture with its object `base`, replacing only the receiving `event` declaration. For Markdown the same declarations are placed under a frontmatter `$schema`. `b|` and `th|` mark the cursor; the marker is removed before the request. “No items” means an empty completion result. Every row was measured through the provider's returned LSP completion items; the folded-frontmatter spread was additionally checked through an LSP session.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F1.1 | Standalone spread | Closed multiline single quote, `...b| }'` | `base`; clean | `base` |
| R2-F1.2 | Frontmatter spread | Same closed single quote | `base`; clean | `base` |
| R2-F1.3 | Standalone spread | Closed multiline double quote | `base`; clean | `base` |
| R2-F1.4 | Frontmatter spread | Same closed double quote | `base`; clean | `base` |
| R2-F1.5 | Standalone spread | Multiline single quote still open, `...b|` | No items | `base` |
| R2-F1.6 | Frontmatter spread | Same open single quote | No items | `base` |
| R2-F1.7 | Standalone spread | Multiline double quote still open | No items | `base` |
| R2-F1.8 | Frontmatter spread | Same open double quote | No items | `base` |
| R2-F1.9 | Standalone spread | Folded block scalar `>-` | `base`; clean | `base` |
| R2-F1.10 | Frontmatter spread | Folded block scalar `>-` | No items, also confirmed by session | `base` |
| R2-F1.11 | Standalone spread | Literal block scalar `|-` | `base`; clean | `base` |
| R2-F1.12 | Frontmatter spread | Literal block scalar `|-` | No items | `base` |
| R2-F1.13 | Standalone scalar `$extends` | Plain `b|` | `base`; clean | `base` |
| R2-F1.14 | Frontmatter scalar `$extends` | Plain `b|` | `base`; clean | `base` |
| R2-F1.15 | Standalone scalar `$extends` | One-line single-quoted `b|` | `base`; clean | `base` |
| R2-F1.16 | Frontmatter scalar `$extends` | Same single quote | `base`; clean | `base` |
| R2-F1.17 | Standalone scalar `$extends` | One-line double-quoted `b|` | `base`; clean | `base` |
| R2-F1.18 | Frontmatter scalar `$extends` | Same double quote | `base`; clean | `base` |
| R2-F1.19 | Standalone scalar `$extends`, including eligibility decision | Folded scalar `>-`, `b|` | `boolean`, `boolean[]`, `boolish`, `boolish[]`, `base` | Only `base` |
| R2-F1.20 | Frontmatter scalar `$extends` | Same folded scalar | No items | `base` |
| R2-F1.21 | Standalone scalar `$extends`, including eligibility decision | Literal scalar `|-`, `b|` | Same five ordinary-type suggestions | Only `base` |
| R2-F1.22 | Frontmatter scalar `$extends` | Same literal scalar | No items | `base` |
| R2-F1.23 | Standalone scalar `$extends` | Single-quoted `base@` / `th|` continuation | No items | `this` |
| R2-F1.24 | Frontmatter scalar `$extends` | Same wrapped single quote | No items | `this` |
| R2-F1.25 | Standalone scalar `$extends` | Double-quoted `base@` / `th|` continuation | No items | `this` |
| R2-F1.26 | Frontmatter scalar `$extends` | Same wrapped double quote | No items | `this` |
| R2-F1.27 | Standalone flow-list `$extends` item | Single-quoted `base@` / `th|` continuation | `this`; clean | `this` |
| R2-F1.28 | Frontmatter flow-list `$extends` item | Same quoted continuation | `this`; clean | `this` |
| R2-F1.29 | Standalone block-list `$extends` item | Single-quoted `base@` / `th|` continuation | No items | `this` |
| R2-F1.30 | Frontmatter block-list `$extends` item | Same quoted continuation | No items | `this` |
| R2-F1.31 | Standalone flow-list `$extends` item | Unquoted `b|` on continuation line | `base`; clean | `base` |
| R2-F1.32 | Frontmatter flow-list `$extends` item | Same flow continuation | `base`; clean | `base` |
| R2-F1.33 | Standalone block-list `$extends` item | Ordinary `- b|` | `base`; clean | `base` |
| R2-F1.34 | Frontmatter block-list `$extends` item | Same block item | `base`; clean | `base` |

The sweep follows the recovered context through both decisions that use it: dispatch to the operand reader and filtering to eligible object types. Rows 19 and 21 demonstrate that merely obtaining some suggestions is not sufficient. The existing passing controls also check replacement ranges, receiver exclusion, root-union scope, and imported-name eligibility; retain those while repairing continuation routing.

**Recommended repair:** recover the owning scalar and its schema position using the shared YAML decoding/cursor facilities for supported scalar presentations, then route `$extends` through the composition operand reader regardless of whether the source model loaded. Preserve partial-quote support when the user has not typed the closing quote. Reuse the current eligibility and authored-range logic. Add provider assertions for the failed rows and small LSP session controls for the distinct spread/directive routes. This is bounded to already supported scalar presentations; it does not require admitting arbitrary YAML syntax or introducing another expression parser.

## Blocked Findings

None.

## Recurrence

R2-F1 belongs to the same class as **review-1.md, R1-F1: Completion disappears in continued flow-sequence operands**. It is a **sibling the earlier sweep missed**, not an unchanged instance of its flow-list reproduction. The repair correctly fixed those original cases and added closed wrapped-quote controls, but did not sweep scalar `$extends`, wrapped block-list operands, open wrapped quotes, or frontmatter block-scalar spreads through context recovery and eligibility filtering. The table above inventories those sites and their clean counterparts together. The requirement has a concrete boundary: the scalar presentations already accepted by the schema grammar plus incomplete quoted operands while editing.

## Input robustness and verification level

The parser review used the existing public-result matrix. Invalid `$extends` shapes produce an error at the value/item; absence means no composition; an empty list is rejected, while an empty object target is valid. The shared validator is covered for absent, explicit/implicit null, wrong carrier type, mixed/all-invalid list elements, empty string/list, and valid scalar/list controls. Root, nested, named-declaration, and union-object routes have valid and invalid controls. The schema and trigger loaders cover duplicate keys, invalid trailing content, second documents, and YAML-specific presentations. Multiline scalar projections include LF and CRLF. The table-mode repair has positive and invalid controls through envelope admission, declaration validation, trigger matching, and DMLS diagnostics.

A scan for defaulted load-bearing fields, `Option` ambiguity, silent element filtering, `unwrap_or_default`, and swallowed parse errors in the composition parser/resolver and repaired source-locator paths found no additional invalid-input acceptance. Optional location failures remain diagnostic fallbacks rather than successful composition. The new finding concerns editor text/cursor handling, so it does not require a separate configuration field-shape matrix.

| User-facing requirement | Strongest checked verification | Result |
| --- | --- | --- |
| Syntax, override order, duplicates, object positions, invalid operands | L1 public parser/factory results | Pass |
| Whole-property replacement, patterns, receiver constraints, example/default/template origins, skipped overwritten members | L1 public resolution, lowering, validation, and temporary-file fixtures | Pass |
| Local/file/embedded lookup, aliases, recursion, depth, dependencies | L1 public resolver/validation and embedded runtime/prepared fixtures | Pass |
| JSON Schema equivalence, strict/soft validation, subtype, authored export round trips | L1 public API comparisons | Pass |
| Editor completion, hover, definition, operand diagnostics, unsaved import refresh | L1 provider and LSP-session requests | Original cases pass; scalar completion fails under R2-F1 |
| Copied-member lowering diagnostic origin | L1 public library error assertion plus DMLS routing inspection | Pass at the library boundary; no direct DMLS test of that particular error variant |

L1 is appropriate for these language and editor-protocol requirements. None requires a terminal renderer, browser, or physical key event. Both new simplified-schema modules and the DMLS session module are compiled by declared `l1` targets; their names do not strand them in another tier. `just check-tier-coverage simplified-schema` reports zero stranded tests.

Review verification:

- Simplified-schema `just test`: **2,483/2,483** in the default/work-counters/terminal build, **2,103/2,103** in the minimal build. No test reached five seconds; the canonical recorder also reports none at or above five seconds.
- DMLS `cargo nextest run -p darkmatter-lsp --lib --test l1 -E 'test(composition) | test(claudine_schema_discovery) | test(schema_yaml_spellings) | test(a_mapping_schema_value) | test(meta_schema_hover)'`: **92/92**, none slow.
- Darkmatter focused shipped-corpus and type-table registry run: **13/13**, none slow.
- `cargo check --locked -p claudine -p claudine-cli -p darkmatter-cli --tests`: succeeds against the final repaired sources, without temporary restoration.
- Claudine focused schema and host-publication run: **815/815**; Darkmatter CLI `claudine_schema_bundle`: **11/11**. No test reached five seconds. These exercise the restored sources beyond compilation.
- Temporary provider probes measured all 34 table entries in 0.089 and 0.156 seconds. Their test functions logged results rather than asserting correctness; their green status is measurement evidence only. The separate LSP session assertion **failed as expected**, reproducing R2-F1 in 0.064 seconds. All temporary source additions were removed after measurement.

The repair's earlier full-suite/lint/shipped-generator results were read as historical evidence, not substituted for the independent checks above. Cross-OS proof remains CI's responsibility and is not a finding. No formatter, commit, terminal window, or browser was run.

## Implementation record corrections and decisions

The historical record's current-tense multiline-loader limitation was corrected in place and annotated with dated verification evidence. The repair log's broad wrapped-quote completion claim was narrowed to closed quotes and annotated with this review's counterexamples. These are record corrections, not additional findings.

The restored Claudine compatibility type source and removal of the duplicate incomplete shipped lifecycle draft preserve the existing supported trigger contract. The registration draft remains available, and host registration is explicitly outside this spec. Keeping this repair is the recommendation already supported by review-1.md; it needs neither another ruling nor a post-cycle confirmation request. No other implementation choice meets the threshold for interrupting the author.
