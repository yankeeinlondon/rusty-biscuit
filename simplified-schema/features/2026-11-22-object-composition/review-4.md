---
$schema: feature-review.yaml
ready: true
findings: []
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
reviewed-by: codex/gpt-6.1-sol
recurrence: false
created: "2026-11-22T20:32:47+00:00"
spec: 2026-11-22-object-composition/spec.md
implemented: false
description: "A **feature** review of `2026-11-22-object-composition/spec.md`"
feature: 2026-11-22-object-composition/review-4.md
previous: 2026-11-22-object-composition/review-3.md
---

# Object composition review, iteration 4

The feature is **production ready** under the specification's contract. All required instances of the previous findings are verified fixed. No new findings, partial repairs, regressions, or blocked work remain.

## Previous Finding Verification

Read review-3.md and the repair's Finding Ledger before reviewing the latest changes. Also checked both earlier reviews and their repair inventories. None contains a Human rulings section. The accepted restoration of Claudine's sequence schemas remains unchanged; there is no new reason to revisit it.

The inventory below includes clean controls and additional sibling rows from the ledgers, rather than relying on parent completion flags. The source and registered assertions were inspected, then exercised through the runs listed below.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R3-F1: Explicit properties on scalar continuation lines hide spread completion context | R3-F1.1–R3-F1.41 and R3-F1.S1–S2 | **verified fixed** | Rows 1–32 map to the first 16 `EXPLICIT_PROPERTY_OPERANDS` entries, each checked standalone and in frontmatter with exact labels and edit ranges. Rows 33–36 assert all four receiving scopes. Rows 37–38 exercise scalar/list `$extends`, including directives after properties containing misleading text. Rows 39–40 assert eight navigation and eight diagnostic cases. Row 41 checks real sibling boundaries, data values, table entries, and closing quotes. The final two table entries cover S2's `#` text; the ordinary data-completion test covers S1. All these tests and the folded-spread LSP session assertion pass in the 104-test editor run; CRLF and last-good-buffer controls also pass. | None. The newly found siblings from review #3 are now repaired, not merely the earlier continuation cases. |
| R2-F1: Scalar continuation completion still loses composition context | R2-F1.1–R2-F1.34 and R2-F1.S1–S8 | **verified fixed** | Inspected all 17 `CONTINUED_OPERANDS` entries in both hosts, further scalar presentations, empty operands, navigation/errors, and negative controls. They pass in the editor run. The shared scalar-prefix reader's presentation and outside-scalar tests pass in both library builds; the sequence-sibling regression test passes in the editor run. | None. |
| R1-F1: Completion disappears in continued flow-sequence operands | R1-F1.1–R1-F1.11 and R1-F1.S1–S5 | **verified fixed** | Continued flow items, empty/quoted/reference items, frontmatter and union-arm scopes, same-line/scalar/block controls, receiver exclusion, eligibility, and unrelated-list controls pass. The LSP session checks completion, navigation, and errors on continued operands. | None. |
| R1-F2: Multiline quoted object definitions fail source projection | R1-F2.1–R1-F2.8 and R1-F2.S1–S3 | **verified fixed** | Public loading/lowering compares both quote styles with ordinary and composed one-line objects; wrapped `$extends` references retain their spans. The location matrix covers 15 presentations with LF/CRLF, and suggestion-span and DMLS grammar-range assertions pass. The shared source locator reads the whole scalar. | None. |
| R1-F3: Deleted sequence type source still prevents consumers from compiling | R1-F3.1–R1-F3.8 | **verified fixed** | Restored `sequence-types.yaml` supplies the production bundle, CLI tests, host-publication tests, embedded-schema tests, and corpus/discovery includes. Checking Claudine, its CLI, and the Darkmatter CLI with `--tests` succeeds. Frozen benchmark inputs remain intact; shipped corpus and editor discovery controls pass. | None. |
| R1-F4: Shipped Claudine triggers no longer satisfy their consumers | R1-F4.1–R1-F4.7 | **verified fixed** | Sequence/type sources use the supported trigger envelope; the incomplete shipped lifecycle duplicate is absent while its registration draft remains. Both library feature builds pass their shipped-bundle tests. All 12 Darkmatter shipped-corpus tests and DMLS discovery controls pass. Self-contained embedded composition tests check runtime and prepared results. | None. Host registration remains outside this feature. |
| R1-F5: Envelope admission reserves `$extends` inside type tables | R1-F5.1–R1-F5.8 and R1-F5.S1–S5 | **verified fixed** | Table parsing, the schema-option validator, trigger matching, and DMLS table classification preserve declaration-table semantics. Public factory tests cover primitive/object/union/alias declarations named `$extends` across envelopes. Object-position reservation, table-key rejection, registry validation, and DMLS location/hover controls pass. | None. |

## Unblocked Findings

None.

## Blocked Findings

None.

## Contract and implementation review

The latest repair makes the shared scalar decoder decide whether an earlier value contains the cursor before the editor interprets a `key:`-looking line as a new mapping entry. In the Darkmatter language-server package, [scalar_holders](../../../darkmatter/dmls/src/providers/frontmatter.rs) supplies that ownership to cursor routing, receiving-object scope, and tolerant namespace recovery. The decoder also decides whether a comment-looking line is scalar text. Real siblings and text after a closing quote retain ordinary behavior. The repair's comments and current editor documentation describe these boundaries consistently.

Reviewed the broader composition contract as well as the latest repair. In the SimplifiedSchema package, [object assembly](../../lib/src/resolve/composition.rs) resolves every operand before choosing surviving members, preserves each winner's declaring namespace and source, and replaces patterns by their emitted identity against the final literal keys. Composition and alias hops consume the reference-depth budget without pretending to descend into child values. Member assembly does not expand recursive children. The corresponding public API tests check ordering, whole-member replacement, skipped examples/templates, source attribution, receiver constraints, recursion, and ineligible targets.

Authored export preserves composition syntax; unresolved lowering fails. Resolved lowering, strict/soft validation, queries, and subtype comparisons are tested against handwritten equivalents. File and embedded tests check transitive dependencies and imported-name scope. Editor tests exercise unsaved imported edits, missing files becoming available, operand locations, and name/import completion. No additional defect was found in these paths.

## Input robustness

Default outcome: an invalid `$extends` carrier or operand is rejected with an error identifying its value or sequence item; it never becomes an empty contribution. Exceptions are defined: an absent directive means no composition, a valid scalar or nonempty sequence composes, repeated references are allowed, and a reference to an explicitly empty object contributes no members.

Checked the shared field validator's assertions for explicit and implicit null, number/boolean/mapping carriers, mixed-invalid and all-invalid sequences, empty strings/lists, and invalid operand suffixes. Positive controls exercise scalar, flow-list, and block-list values. Root, nested, named-type, root-union, property-union, and Markdown routes establish that the shared validation is wired into each supported position. These assertions ran in the library suites.

Standalone-schema and trigger-envelope loader tests separately reject duplicate keys, invalid trailing content, and multiple YAML documents. The multiple-document error identifies the document-level problem rather than an individual field. Supported multiline quoting, folding, escapes, flow collections, and CRLF are covered by loading and source-projection tests.

Scanned the composition parser, assembler, scalar decoder, and editor routing for defaulted load-bearing fields, ambiguous optional carriers, silently filtered invalid items, `unwrap_or_default`, and swallowed parse errors. The parser checks YAML variants explicitly. Optional scalar/cursor results support editor recovery; they do not make invalid schemas resolve successfully. The assembler's fallback in non-failing metadata walks is paired with error propagation during expansion. No additional acceptance defect was found.

## Verification level and evidence

| User-facing requirement | Strongest checked level | Result |
| --- | --- | --- |
| Spread ordering, duplicates, directive shapes and object positions | L1 public parser/factory results | Pass |
| Whole-property and pattern replacement; receiver constraints; skipped work and declaring origins | L1 resolution, validation, and temporary-file fixtures | Pass |
| Local/file/embedded names, aliases, recursion, depth, and dependencies | L1 public resolver and embedded-bundle results | Pass |
| Handwritten equivalence, strict/soft validation, subtyping, and authored round trips | L1 public API comparisons | Pass |
| Completion, scope, eligibility, and replacement ranges while typing | L1 provider results and LSP protocol sessions | Pass |
| Hover, definition, operand diagnostics, and unsaved import refresh | L1 provider results and LSP protocol sessions | Pass |

These requirements concern language semantics and editor protocol results. L1 exercises their relevant boundary; none requires terminal rendering, browser output, or OS keyboard injection. The new unit tests compile under the library target, editor session tests are declared in the consolidated L1 target, and their names do not exclude them from L1. Feature-enabled library tests run with the features declared for CI. `just check-tier-coverage simplified-schema` reports zero stranded tests.

Independent runs in this review:

- SimplifiedSchema `just test`: **2,485/2,485** with work-counters/terminal, then **2,105/2,105** without default features. Longest individual test: **0.540 s**. Run IDs: `6960504c-98b5-41ed-98fe-3d93a1b0a022`, `b1408a72-8dea-40d0-9c70-151854faa291`.
- `cargo nextest run --locked -p darkmatter-lsp --lib --test l1 -E 'test(composition) | test(schema_yaml_spellings) | test(claudine_schema_discovery) | test(key_like_line_on_a_data_scalar)'`: **104/104 selected**, longest **0.913 s**. Run ID: `250f0774-084c-4226-890a-091a3604a68e`.
- `cargo nextest run --locked -p darkmatter --test l1 -E 'test(shipped_schema_corpus) | test(a_types_table_entry_named_extends) | test(schemas_named_type_merge) | test(schema_language_ownership_guard)'`: **56/56 selected**, longest **1.081 s**. Run ID: `265a7a9e-218e-4b71-8c82-c29a90987eed`.
- `cargo check --locked -p claudine -p claudine-cli -p darkmatter-cli --tests`: succeeds.

No test in these runs reached five seconds. The repair's recorded broader suites and lint checks were read as historical evidence, not represented as fresh runs. Cross-OS evidence remains CI's responsibility. No formatter, commit, terminal window, or browser was run.

## Recurrence

No finding recurs in this review. The previously recurring completion-context class is verified fixed across the complete required inventory, including the explicit-property siblings identified in review #3.

## Implementation records and decisions

No verified false current behavior claim required another correction to `log.md` or `implementation-log.md`. Their earlier limitations are already qualified by dated review corrections or subsequent repair sections. The latest completion report agrees with the implementation and the fresh results above.

No decision requires an author ruling or post-cycle confirmation. Review #3's implementation claim and implementer identity are preserved; this review records the independent assessment. The specification's review count is updated to four and `completed` is set to true. The feature directory remains in place for the author's review process.
