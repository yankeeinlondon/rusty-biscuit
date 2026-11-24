---
$schema: feature-review.yaml
ready: false
findings:
  - "R3-F1 (high): Explicit properties on scalar continuation lines hide spread completion context"
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
reviewed-by: codex/gpt-6.1-sol
recurrence: true
created: "2026-11-22T20:11:01+00:00"
spec: 2026-11-22-object-composition/spec.md
implemented: true
description: "A **feature** review of `2026-11-22-object-composition/spec.md`"
feature: 2026-11-22-object-composition/review-3.md
previous: 2026-11-22-object-composition/review-2.md
log: /home/build/wt/rusty-biscuit/fix-biscuit-terminal/simplified-schema/features/2026-11-22-object-composition/log.md
implemented-by: claude/opus
next: 2026-11-22-object-composition/review-4.md
---

# Object composition review, iteration 3

The feature is **not production ready**. Every required instance from the previous reviews is verified fixed. One newly found sibling remains: an explicit property on a scalar continuation line can make the editor forget that a later `...name` is a composition operand. No finding requires a human decision.

## Previous Finding Verification

Read review-2.md and the repair's `log.md` Finding Ledger before checking new work. Also read review-1.md and the original implementation record. Neither earlier review contains a `## Human rulings` section. The earlier recommendation to retain the restored Claudine compatibility schemas remains accepted; no new evidence changes its premise.

The child rows below identify the required inventory, including the repair's additional sweep rows. Their claims were checked against the implementation and the assertions in registered tests, then exercised in the runs recorded below.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R2-F1: Scalar continuation completion still loses composition context | R2-F1.1–R2-F1.34 and R2-F1.S1–S8 | **verified fixed** | Inspected all 17 entries in `CONTINUED_OPERANDS`, each served standalone and as frontmatter with exact labels and edit ranges. The open-frontmatter cases explicitly serve a last-good buffer first. Ran `composition_operands_on_scalar_continuation_lines`, CRLF controls, further presentations, empty operands, negative controls, navigation, diagnostics, the sequence-sibling regression, and the LSP session controls. All pass in the 96-test editor run. The shared scalar-prefix decoder's tests pass in both SimplifiedSchema builds. | No listed instance remains. R3-F1 is a newly found sibling involving an explicit property before the spread, not an unchanged original instance. |
| R1-F1: Completion disappears in continued flow-sequence operands | R1-F1.1–R1-F1.11 and R1-F1.S1–S5 | **verified fixed** | Inspected the continued-flow scanner and routing. Continued lists, empty/quoted/reference items, frontmatter and union-arm controls, eligibility, receiver exclusion, wrapped definitions, and the corresponding LSP session pass in the editor run. | None of the original instances; R3-F1 repeats the broader context-recovery class. |
| R1-F2: Multiline quoted object definitions fail source projection | R1-F2.1–R1-F2.8 and R1-F2.S1–S3 | **verified fixed** | Public factory/lowering comparisons, the wrapped `$extends` reference and its spans, 15 source presentations with LF/CRLF, suggestion spans, and DMLS grammar ranges are asserted in the passing library/editor suites. Inspected the whole-scalar source locator. | None. R3-F1 concerns completion routing, not loading or source projection. |
| R1-F3: Deleted sequence type source still prevents consumers from compiling | R1-F3.1–R1-F3.8 | **verified fixed** | Restored `sequence-types.yaml` and its production, CLI-test, host-publication, embedded-schema, corpus, and discovery includes are present. `cargo check --locked -p claudine -p claudine-cli -p darkmatter-cli --tests` succeeds, compiling the affected includes. Frozen benchmark inputs retain their existing source. Corpus/discovery tests also pass. | None. |
| R1-F4: Shipped Claudine triggers no longer satisfy their consumers | R1-F4.1–R1-F4.7 | **verified fixed** | The sequence/type split uses the supported envelope; the incomplete shipped lifecycle duplicate is absent and its registration draft remains. Shipped-bundle tests pass in both library builds, all 12 Darkmatter shipped-corpus tests pass, and DMLS removed-key diagnostics and composition editor fixtures pass. | None. Host registration remains outside this spec. |
| R1-F5: Envelope admission reserves `$extends` inside type tables | R1-F5.1–R1-F5.8 and R1-F5.S1–S5 | **verified fixed** | Inspected table-mode parsing, `SchemaOptions::judge`, trigger matching, DMLS table classification and error location. Public factory tests exercise primitive/object/union/alias declarations named `$extends` across envelopes; table-key rejection and object-position reservation controls pass in both library builds. The Darkmatter registry control and DMLS table/hover tests pass. | None. |

## Unblocked Findings

### R3-F1 (high): Explicit properties on scalar continuation lines hide spread completion context

**Authority — spec:** inline spreads may appear “at the start, middle, or end of an inline object body,” and the editor must offer name/import completions after `...`; “Completion should work while the operand is incomplete.” A normal wrapped object with an explicit property before its spread offers primitive types that cannot be composed, or loses the expected object/import completion.

**Class:** line-based YAML key detection treats a property inside a continuing scalar as a new YAML mapping entry, bypassing scalar ownership recovery and consequently the composition cursor and eligibility decisions.

Request completion immediately after `b` in this valid Markdown frontmatter:

```markdown
---
$schema:
  base: '{ id: string }'
  event: >-
    { at: string,
      next: number, ...b }
---
```

The returned labels are `base`, `boolean`, `boolean[]`, `boolish`, and `boolish[]`; only `base` is eligible. With `...base@th` at that position, no items are returned; `this` should be offered. This requires neither unusual syntax nor an invalid YAML document. A separate in-memory LSP session reproduced the five-label result through `textDocument/completion` in 0.047 seconds.

In the Darkmatter language-server package, [line_cursor](../../../darkmatter/dmls/src/providers/frontmatter.rs:447), which finds the schema value containing the cursor, calls its per-line mapping reader before trying scalar continuation. The [continued_line_holder](../../../darkmatter/dmls/src/providers/frontmatter.rs:510) helper also refuses a current line with a mapping separator. Its [backward holder scan](../../../darkmatter/dmls/src/providers/frontmatter.rs:528) can select an earlier scalar-content line such as `next: number,` as the holder instead of reaching `event: >-`. Both paths miss the existing shared decoder that could establish the real scalar boundary.

Following this value into its consumers matters: [authoring_key_path](../../../darkmatter/dmls/src/providers/frontmatter.rs:387) supplies the receiving object's scope, and [continued_value_anchor](../../../darkmatter/dmls/src/providers/frontmatter.rs:501) supplies the range blanked while recovering local names from an incomplete document. With the wrong holder, standalone open quotes lose `base` as well as composition filtering. A loaded standalone source model supplies a correct fallback for closed scalars; frontmatter does not get that fallback. The library decoder itself is not rejecting these valid scalars.

#### Complete sibling sweep

The first 32 rows reuse the repair's `library` fixture and `operand_case_mismatches` helper. Only the receiving definition is changed. Each row was read through the provider's returned completion labels and edit ranges. Open frontmatter quotes were served over their closed last-good buffer, exactly as in the repair tests. The four further owner cases use small frontmatter fixtures based on the existing schema/type-definition controls.

For compactness, **extra primitive suggestions** means the exact labels `boolean`, `boolean[]`, `boolish`, `boolish[]`, and `base`. **Primitive suggestions only** means the first four of those, with `base` missing. Every name case expects exactly `base`; the reference-half cases expect exactly `this`. Clean rows returned the expected edit range as well as labels.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R3-F1.1 | Standalone spread | Closed single quote; continuation `next: number, ...b` | `base`; clean | `base` |
| R3-F1.2 | Frontmatter spread | Same closed single quote | Extra primitive suggestions | `base` |
| R3-F1.3 | Standalone spread | Closed double quote; same continuation | `base`; clean | `base` |
| R3-F1.4 | Frontmatter spread | Same closed double quote | Extra primitive suggestions | `base` |
| R3-F1.5 | Standalone spread | Single quote still open; same continuation | Primitive suggestions only | `base` |
| R3-F1.6 | Frontmatter spread | Same open single quote, last-good tree | Extra primitive suggestions | `base` |
| R3-F1.7 | Standalone spread | Double quote still open; same continuation | Primitive suggestions only | `base` |
| R3-F1.8 | Frontmatter spread | Same open double quote, last-good tree | Extra primitive suggestions | `base` |
| R3-F1.9 | Standalone spread | Folded `>-`; same continuation | `base`; clean | `base` |
| R3-F1.10 | Frontmatter spread | Same folded scalar | Extra primitive suggestions | `base` |
| R3-F1.11 | Standalone spread | Literal `|-`; same continuation | `base`; clean | `base` |
| R3-F1.12 | Frontmatter spread | Same literal scalar | Extra primitive suggestions | `base` |
| R3-F1.13 | Standalone spread | Folded scalar; `next: number,` on earlier line, then a more-indented `...b` | `base`; clean | `base` |
| R3-F1.14 | Frontmatter spread | Same earlier property line | Extra primitive suggestions | `base` |
| R3-F1.15 | Standalone spread | Closed single quote; earlier property line as above | `base`; clean | `base` |
| R3-F1.16 | Frontmatter spread | Same earlier property line | Extra primitive suggestions | `base` |
| R3-F1.17 | Standalone array-item spread | Folded `{ at: string,` / `next: number, ...b }[]` | `base`; clean | `base` |
| R3-F1.18 | Frontmatter array-item spread | Same array item | Extra primitive suggestions | `base` |
| R3-F1.19 | Standalone union-arm spread | Folded `{ at: string,` / `next: number, ...b } \| string` | `base`; clean | `base` |
| R3-F1.20 | Frontmatter union-arm spread | Same union arm | Extra primitive suggestions | `base` |
| R3-F1.21 | Standalone reference-half completion | Folded continuation `next: number, ...base@th` | `this`; clean | `this` |
| R3-F1.22 | Frontmatter reference-half completion | Same continuation | No items | `this` |
| R3-F1.23 | Standalone spread control | Single quoted object all on one line, property before spread | `base`; clean | `base` |
| R3-F1.24 | Frontmatter spread control | Same one-line single quote | `base`; clean | `base` |
| R3-F1.25 | Standalone spread control | One-line double quote, property before spread | `base`; clean | `base` |
| R3-F1.26 | Frontmatter spread control | Same one-line double quote | `base`; clean | `base` |
| R3-F1.27 | Standalone spread control | Entire object on first folded-scalar content line | `base`; clean | `base` |
| R3-F1.28 | Frontmatter spread control | Same first folded content line | `base`; clean | `base` |
| R3-F1.29 | Standalone spread control | Entire object on first literal-scalar content line | `base`; clean | `base` |
| R3-F1.30 | Frontmatter spread control | Same first literal content line | `base`; clean | `base` |
| R3-F1.31 | Standalone nested-object spread | Folded `{ nested: { at: string,` / `next: number, ...b } }` | `base`; clean | `base` |
| R3-F1.32 | Frontmatter nested-object spread | Same nested object | Extra primitive suggestions | `base` |
| R3-F1.33 | Frontmatter `type-definition` value | `shape: >-` containing the same two-line object, local `base` in `$schema` | Extra primitive suggestions | `base` |
| R3-F1.34 | Object inside frontmatter `schema` value | `shape.event: >-` containing that object | Extra primitive suggestions | `base` |
| R3-F1.35 | Declaration inside frontmatter `schema(mapping)` value | `shape.event: >-` containing that object | Extra primitive suggestions | `base` |
| R3-F1.36 | Implicit root-union arm scope | Frontmatter arm containing `base` and the folded receiving `event` | Extra primitive suggestions | Only arm-local `base` |
| R3-F1.37 | `$extends` scalar operand route and eligibility | Plain, quoted, wrapped quoted, folded and literal values; standalone/frontmatter | Clean in the previous finding's rerun matrix | Eligible object names or reference completions |
| R3-F1.38 | `$extends` sequence operand route and eligibility | Continued flow items, wrapped quoted flow/block items, ordinary block items; both hosts | Clean in the previous finding's rerun matrix | Eligible object names or reference completions |
| R3-F1.39 | Operand hover and go-to-definition projection | Completed `...base` after the explicit property in closed single/double quotes and folded/literal scalars, both hosts | All eight cases find the local declaration | Local declaration |
| R3-F1.40 | Operand diagnostic projection | Replace only that operand with primitive `label` in those eight cases | All eight diagnostics cover exactly `label` and explain ineligibility | Located invalid-target error |
| R3-F1.41 | Non-composition contexts and scalar boundaries | Data values, declaration-table key `$extends`, text after closing quote, sibling property after block scalar, sequence sibling after empty holder | Clean in the existing negative-control tests | Ordinary completion; no leaked composition context |

This inventories context discovery, scalar-holder selection, namespace recovery, dispatch to the composition cursor, eligibility filtering, reference-half completion, and the independent navigation/diagnostic projections. The isolated fixture used for diagnostic assertions omits unrelated alias-cycle declarations from the larger completion fixture so that its operand error is observable.

**Recommended repair:** establish whether an earlier scalar actually contains the cursor before interpreting colons on its content lines as YAML mapping separators. Reuse the shared scalar decoder and recover the actual holder for scope selection and tolerant namespace recovery too. Preserve genuine YAML sibling boundaries and table/data distinctions. Extend the existing assertion tables with the listed cases and retain one LSP-session assertion. This needs no new syntax or escaping layer; the supported quoted and block scalar boundaries already supply the needed distinction. Review affected comments alongside the fix, especially comments assuming that every `key:`-looking continuation line starts a new node.

## Blocked Findings

None.

## Recurrence

R3-F1 repeats the context-recovery class of **review-1.md, R1-F1: Completion disappears in continued flow-sequence operands**, and **review-2.md, R2-F1: Scalar continuation completion still loses composition context**. It is a **sibling the earlier sweeps missed**. Every original required case now passes; there is no evidence that the latest repair regressed one of those cases.

The prior sweeps put the spread at the beginning of the continuation line. They should also have followed a spread after an explicit property through the per-line mapping reader, backward scalar-holder scan, receiving-object scope, tolerant namespace anchor, composition cursor, and eligibility filter. The new rows cover those sites together, including the case where the misleading property is on an earlier continuation line. The boundary remains ordinary schema objects in the scalar presentations already supported by the shared decoder. The failing middle-position spread and incomplete operand are explicitly required, so this finding remains high priority.

## Input robustness and verification level

Reviewed the existing public-result matrix rather than adding duplicate format tests for an editor-routing defect. Default outcome: invalid composition fields fail with a located error; they do not become an empty member set. Exceptions: absent `$extends` means no composition, a valid scalar or nonempty list composes, and a named explicit empty object is an eligible target. Null (including YAML's implicit null), wrong carrier types, mixed/all-invalid sequence items, empty string/list, invalid operand suffixes, and supported scalar/list controls have assertions. Root, nested, named-type, root-union and property-union positions exercise the shared validator. Standalone and trigger loader tests reject duplicate keys, invalid trailing content, and multiple documents. Quoting, folding, escapes, and CRLF are covered through source projection and public loading.

The parser/assembler scan for defaulted load-bearing fields, ambiguous `Option` use, filtered-out malformed items, `unwrap_or_default`, and swallowed parse results found no additional field-shape acceptance defect. Optional source locations and tolerant editor recovery do not turn an invalid composition into successful resolution. The latest helper reads editor text and a cursor, not a new configuration format; a separate field-shape matrix would not test R3-F1.

| Requirement | Checked verification level | Result |
| --- | --- | --- |
| Spread positions/order, duplicates, `$extends` values and object positions | L1 public parser/factory results | Pass |
| Whole-property replacement, patterns, receiver constraints, skipped members, example/default/template origins | L1 public resolution, lowering, validation and temporary-file fixtures | Pass |
| Local/file/embedded names, aliases, eligible targets, recursion, depth, transitive dependencies | L1 resolver/validation and embedded fixtures | Pass |
| JSON Schema, strict/soft validation and subtype equivalence; authored export round trips | L1 public API comparisons | Pass |
| Editor name/import completion, scope and eligibility | L1 provider results and LSP protocol requests | Previous cases pass; R3-F1 fails |
| Hover, definition, operand errors and unsaved import refresh | L1 provider results and LSP protocol requests | Pass, including new explicit-property navigation/error controls |
| Copied declaration/default origin | L1 public library error/default assertions | Pass; no direct DMLS test for every library error variant is required by this contract |

These are language and editor-protocol requirements. L1 directly exercises the relevant boundary; no terminal-rendering, browser, or OS key-injection requirement needs L2 or L3 here. Tests are declared in consolidated L1 targets; the relevant module names carry no excluded tier marker. `just check-tier-coverage simplified-schema` reports zero stranded tests.

Independent verification during this review:

- SimplifiedSchema `just test`: **2,485/2,485** with work-counters/terminal and **2,105/2,105** without default features. No test reached five seconds. The canonical recorder reports zero slow tests for the recorded default build.
- DMLS `cargo nextest run --locked -p darkmatter-lsp --lib --test l1 -E 'test(composition) | test(schema_yaml_spellings) | test(claudine_schema_discovery)'`: **96/96**, none slow (run `2bfcfc48-5db1-441b-b2a5-6f97e4a71698`).
- Darkmatter shipped corpus and type-table registry: **13/13**, none slow.
- `cargo check --locked -p claudine -p claudine-cli -p darkmatter-cli --tests`: succeeds against the repaired sources.
- Temporary provider sweep measured all 32 scalar/host rows in 0.252 seconds, reporting both clean and failing results. A separate LSP-session assertion failed on the five-label result in 0.047 seconds (run `b65dd68b-5c61-4859-b74c-7d7c83fd6613`). A further probe measured four owner routes and asserted all eight navigation/diagnostic cases in 0.132 seconds (run `7557bbb0-0d42-402b-b83b-0681590aa46e`). The reporting probes deliberately end with a failure to print the complete measurements; those exits are evidence collection, not passing tests. The LSP assertion independently demonstrates the defect.

All temporary probe additions were removed and both source files restored byte for byte to their pre-review contents. Historical implementer lint and wider-suite results were read as historical evidence, not rerun claims. Cross-OS proof remains CI's responsibility. No formatter, commit, terminal window, or browser was run.

## Implementation record corrections and decisions

Narrowed the repair completion report's broad assertion that spreads inside continuing scalars complete `base` to the actual tested cases. Added a dated note to `log.md` explaining the explicit-property counterexample, the true observed result, and the provider/LSP verification. This record correction is not a separate finding or human-review item. The original implementation record already labels its repaired limitations historically; no additional false current claim there required correction.

No decision needs an author ruling or post-cycle confirmation. The previous restored-schema decision remains unchanged. The repair recommendation stays within the spec and reuses existing scalar decoding.
