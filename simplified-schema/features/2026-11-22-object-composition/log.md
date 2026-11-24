---
implementation_1: "2026-11-22T18:16:34+00:00"
implementation_2: "2026-11-22T19:46:26+00:00"
implementation_3: "2026-11-22T20:17:09+00:00"
---

# Object composition review/fix log

## Implementation of Review Findings #1

> **started at:** 2026-11-22T18:16:34+00:00

- this implementation is attempting to implement _all_ of the review findings found in '/home/build/wt/rusty-biscuit/fix-biscuit-terminal/simplified-schema/features/2026-11-22-object-composition/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review's `implemented` flag was confirmed `false` before repairs began
- the review has no `human-in-the-loop` items and no blocked findings, so no finding can be blocked on a ruling

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R1-F1 | Completion disappears in continued flow-sequence operands | fixed | all child rows verified; `darkmatter-lsp` lib 255/255 and l1 38/38 scoped; area `just test` 17536/17536 |
| R1-F1.1 | Standalone `$extends: [base,` continuation `b` → `base` | fixed | `composition_tests::composition_extends_flow_sequence_continued_onto_later_lines`; `object_composition_editor::operands_on_continuation_lines_complete_navigate_and_report` |
| R1-F1.2 | Same continuation, empty item → eligible object names | fixed | same unit test offers exactly the eligible names, closed and unclosed |
| R1-F1.3 | Same continuation, `base@` → `this` | fixed | same unit test offers `this` |
| R1-F1.4 | Same continuation, single-quoted `b` → `base` | fixed | same unit test, quoted `'b'` |
| R1-F1.5 | Markdown frontmatter continued flow item → `base` | fixed | `composition_continued_flow_sequence_in_frontmatter_and_union_arms` and the L1 session test |
| R1-F1.6 | Scalar `$extends: b` → `base` (control) | clean | `composition_extends_scalar_plain_and_quoted` passes |
| R1-F1.7 | Block-list item `- b` → `base` (control) | clean | `composition_extends_block_sequence_item` passes |
| R1-F1.8 | Same-line flow item `[base, b]` → `base` (control) | clean | `composition_extends_flow_sequence_items` passes |
| R1-F1.9 | Root-union mapping arm `$extends: b` → arm-local `base` (control) | clean | `composition_root_union_arm_scope_is_the_cursor_arm` passes; continued arm form added |
| R1-F1.10 | Property-union mapping arm `$extends: b` → `base` (control) | clean | continued property-union arm asserted in the frontmatter/union-arms test |
| R1-F1.11 | Inline spread `{ ...b }` → `base` (control) | clean | `composition_in_a_frontmatter_declaration`, `composition_spread_offers_only_eligible_objects` pass |
| R1-F2 | Multiline quoted object definitions fail source projection | fixed | all child rows verified; 11 sibling presentations fixed; simplified-schema `just test` 2483 + 2103 passed |
| R1-F2.1 | Single-quoted multiline inline object → loads and lowers | fixed | `object_composition::composition_objects_wrapped_across_lines_load_and_lower_like_one_line` |
| R1-F2.2 | Double-quoted multiline inline object → loads and lowers | fixed | same test |
| R1-F2.3 | Single-quoted multiline `{ ...base, n: number }` → loads and lowers | fixed | same test |
| R1-F2.4 | Double-quoted multiline spread → loads and lowers | fixed | same test |
| R1-F2.5 | One-line quoted object (control) | clean | same test, lowers equal to the wrapped forms |
| R1-F2.6 | Folded `>-` object (control) | clean | same test |
| R1-F2.7 | Single-quoted multiline `$extends` operand `base@`/`this` → loads | fixed | `object_composition::composition_extends_operand_wrapped_inside_its_reference_loads`: YAML value `base@ this` reads as `base@this`; spans on `base` and `this` |
| R1-F2.8 | Folded spread (control) | clean | same wrapped-object test |
| R1-F3 | Deleted sequence type source still prevents consumers from compiling | fixed | all child rows verified; `cargo check --locked --workspace --all-targets` finishes |
| R1-F3.1 | `claudine/lib/src/composition/schema/bundle.rs:29` → builds | fixed | `cargo check --locked -p claudine` (and `--tests`) finishes |
| R1-F3.2 | `darkmatter/cli/tests/l1/claudine_schema_bundle.rs` (11 includes) → compiles | fixed | `cargo check --locked -p darkmatter-cli --tests` finishes; `claudine_schema_bundle` 11/11 |
| R1-F3.3 | `claudine/cli/src/completion/schema_completion/tests.rs:1132` → compiles | fixed | `cargo check --locked -p claudine-cli --tests`; `schema_completion` 57/57 |
| R1-F3.4 | `claudine/lib/tests/l1/host_publication.rs:308` → compiles | fixed | claudine `host_publication` tests pass (842/842 scoped run); claudine `just test` 12690/12690 |
| R1-F3.5 | `claudine/lib/tests/l1/claudine_schemas_embedded.rs:171` → compiles | fixed | same claudine runs |
| R1-F3.6 | `claudine/lib/tests/l1/claudine_schemas.rs:511` → compiles | fixed | same claudine runs |
| R1-F3.7 | Darkmatter benchmark fixtures / DMLS response contracts (control) | clean | frozen bench fixtures untouched; `recursive_schema_golden` and DMLS `sequence`/`nested_span` tests pass |
| R1-F3.8 | Simplified-schema corpus and DMLS discovery fixture lists (control) | fixed | include restored in `shipped_corpus.rs` and `claudine_schema_discovery.rs`; both pass |
| R1-F4 | Shipped Claudine triggers no longer satisfy their consumers | fixed | all child rows verified; shipped artifacts reported current by `scripts/shipped_schemas.py` |
| R1-F4.1 | Simplified-schema shipped bundle test, default and minimal builds → resolves | fixed | `shipped_corpus` 5/5 in default and `--no-default-features` builds; both area builds green in `just test` |
| R1-F4.2 | Darkmatter `shipped_schema_artifacts_parse_passively` → admitted | fixed | darkmatter `shipped_schema_corpus` 12/12 |
| R1-F4.3 | Darkmatter `loadable_shipped_schemas_resolve_and_lower` → resolves | fixed | same run |
| R1-F4.4 | Darkmatter `every_kinded_schema_artifact_passes_its_definition_through_the_registry` → passes | fixed | same run |
| R1-F4.5 | DMLS `removed_sequence_keys_are_reported_at_their_paths` → diagnostic | fixed | `claudine_schema_discovery::removed_sequence_keys_are_reported_at_their_paths` passes |
| R1-F4.6 | Other nine Darkmatter shipped-corpus tests (control) | clean | other nine shipped-corpus tests pass in the same 12/12 run |
| R1-F4.7 | Self-contained composition trigger fixture (control) | clean | composition trigger fixture passes in simplified-schema and DMLS runs |
| R1-F5 | Envelope admission reserves `$extends` inside type tables | fixed | all child rows verified plus sweep rows below |
| R1-F5.1 | Tagged schema `types.$extends: string` → ordinary declaration | fixed | `object_composition::composition_extends_in_a_types_table_loads_through_every_envelope` |
| R1-F5.2 | Mapping definition under `types.$extends` → ordinary object declaration | fixed | same test |
| R1-F5.3 | `[string, number]` under `types.$extends` → ordinary union | fixed | same test |
| R1-F5.4 | Tagged schema with `$schema` and `types` → ordinary declaration | fixed | same test |
| R1-F5.5 | Informal `$schema`/`types` envelope → ordinary declaration | fixed | same test |
| R1-F5.6 | `kind: yaml-schema` type definition → ordinary declaration | fixed | same test and darkmatter `yaml_kind_registry::a_types_table_entry_named_extends_is_an_ordinary_declaration` |
| R1-F5.7 | `$extends: other` alias (control) | clean | same test: an alias of `other` |
| R1-F5.8 | Actual object `$extends` in object positions remains a directive (control) | clean | `composition_extends_stays_a_directive_beside_a_types_table` plus existing reservation tests pass |
| R1-F2.S1 | Sibling presentations: blank line, `''`, `\u0061`, end-of-line `\`, quoted sequence item, flow mapping/sequence across lines, quoted and plain flow items, quoted key with wrapped value | fixed | `located_schema_errors::values_wrapped_across_lines_load_and_locate_their_errors` (15 rows, YAML and frontmatter, LF and CRLF; 11 failed before) |
| R1-F2.S2 | Suggestion-span scanner (`scan_value_scalars`) across quoted/block scalars | fixed | `source_suggestion_spans_follow_quoted_and_block_scalars_across_lines` |
| R1-F2.S3 | DMLS grammar-error range on a wrapped value | fixed | `a_wrapped_quoted_or_flow_value_ranges_its_grammar_error_at_the_token` |
| R1-F1.S1 | Hover, go-to-definition, diagnostic range on continuation-line operands | clean | `object_composition_editor::operands_on_continuation_lines_complete_navigate_and_report` |
| R1-F1.S2 | `...b` completion inside a wrapped quoted object definition | fixed | `composition_spread_in_a_wrapped_quoted_definition` |
| R1-F1.S3 | Eligibility, receiver exclusion, `@this` on continuation lines (namespace blanking) | fixed | continued-sequence unit tests |
| R1-F1.S4 | Receiver offered as its own operand in a `types`-only library | fixed | same-line assertion in the continued test |
| R1-F1.S5 | Unrelated continued lists get no operand completion | clean | `composition_continued_flow_sequence_elsewhere_is_not_an_operand` |
| R1-F5.S1 | Trigger matcher `declares: schema(mapping)` ran object mode first | fixed | `declaration_narrowing::atom_judges::a_mapping_condition_reads_a_table` |
| R1-F5.S2 | `schema(mapping)` keyword judge in validation | fixed | `declaration_narrowing::carriers::mapping_reads_a_table_where_extends_is_a_name` |
| R1-F5.S3 | `$constraints` and pattern keys directly in a `types` table applied to the root or silently dropped | fixed | refused with a located error in every envelope; three `object_composition` tests; DMLS no longer offers them as names |
| R1-F5.S4 | DMLS located `schema(mapping)` frontmatter failures in object mode | fixed | `diagnostics::frontmatter` test `a_mapping_schema_value_is_located_as_a_table` |
| R1-F5.S5 | Existing DMLS hover fixture placed pattern keys directly in a `types` table | fixed | `lsp_session::meta_schema_hover_covers_pattern_keys_inline_and_standalone` moved to the tagged `$schema` envelope; passes |

### Progress

- starting the work on 'R1-F3 and R1-F4 (shipped Claudine schemas)' at 18:20:10
        - handled as one repair because both findings concern the same shipped files (`sequence.yaml`, `sequence-types.yaml`, `lifecycle-events.yaml`)
        - defect class: compile-time or test-time consumers name a schema file that no longer exists, or shipped schema documents have moved ahead of the loader contract that admits them
        - restored `claudine/schemas/sequence-types.yaml` from `8f33de0d7^`; its declarations are identical to the inline `types:` table `8f33de0d7` had introduced, so nothing was lost
        - `sequence.yaml` is back on the trigger contract (`kind`, `match`, `$schema`) and imports `steps(required)@./sequence-types.yaml`; the author's `kind: literal(sequence; required)` match arm, required step list, and newer `template` text are kept
        - removed `claudine/schemas/lifecycle-events.yaml`, a byte-identical duplicate of the `claudine/registration/` draft with no shipped or compiled consumer; the envelope validator is unchanged
        - put back the `sequence-types.yaml` includes the feature had removed from the simplified-schema `shipped_corpus` and DMLS `claudine_schema_discovery` bundle lists
        - regenerated `claudine/lib/generated/shipped.json` with `just shipped-schemas` (it had been stale against the `8f33de0d7` source)
        - docs drift fixed: `claudine/schemas/README.md`, `claudine/docs/topics/schemas.md`, `darkmatter/docs/topics/schemas/dmls-schema-support.md` now describe the `kind: sequence` selection and the required `sequence`; two `sequence.yaml` header comments corrected
        - sweep: every `include_str!`/`include_bytes!` under `claudine/`, `darkmatter/`, `simplified-schema/` targeting `claudine/schemas/*` or `claudine/registration/*` resolves; nothing includes `registration/*` or names `lifecycle-events.yaml`
        - decision: removing the lifecycle duplicate rather than re-kinding it, because no kind admits its `description`/`types` without new behavior and the host-registration plan already removes it; this reverses part of the author's commit `89ffd8678`, so it is listed in the review's `human-review-items`
- work completed for 'R1-F3 and R1-F4 (shipped Claudine schemas)' at 18:31:26
- starting the work on 'R1-F5 (envelope admission reserves `$extends` in type tables)' at 18:31:40
        - defect class: a named-declaration table is validated by an object-mode mapping parser, so object-only reservations leak into the table
        - root cause was the shared `schema(mapping)` declaration-option judge (`format.rs::SchemaOptions::judge`), not `envelope.rs` itself; it served envelope admission, Darkmatter's `kind: schema`/`kind: yaml-schema` validation, and trigger matching
        - repaired: the `mapping` option now parses its carrier in `MappingMode::Table`; bare `schema` and `schema(property(...))` keep object mode
        - sibling fixed: the trigger matcher (`triggers/matcher.rs::scalar_conforms`) ran a bare object-mode `schema` check before the options judge; it now defers to the options judge for `schema(mapping)` atoms
        - sweep, clean: imported `Name@file` tables, inline/embedded/catalog preparation, trigger payload grammar, `KindRegistry::revealed_claims`, DMLS `named_types`/`build_tolerant`, DMLS `$extends` completion, and export already treat table keys as names
        - no shipped payload changed, so no `just shipped-schemas` run or `PREPARATION_REVISION` bump
        - decision: `schema(mapping)` means a table of named declarations (every documented use is a table); alternative `schema(table)` option rejected for its grammar/shipped blast radius; trade-off is parse-only leniency for a `schema(mapping)` field holding a single object
        - doc drift fixed: `objects-and-named-types.md` had described the rejection as behavior; `schema-types.md`, `custom-vocabulary.md`, and the repo skill updated
        - tests added: two `object_composition` and two `declaration_narrowing` L1 cases in simplified-schema, one `yaml_kind_registry` L1 case in Darkmatter; each failed with its repair reverted
        - discovered: `$constraints` inside a `types` table is handled inconsistently (see the residual rows below)
- work completed for 'R1-F5 (envelope admission reserves `$extends` in type tables)' at 18:42:21
- starting the work on 'R1-F2 (multiline quoted object definitions fail source projection)' at 18:42:40
        - defect class: the YAML source locator fails to project spans through a scalar presentation the YAML loader accepts
        - cause: the strict schema-document locator (`simplified/source.rs`) read a quoted value or flow collection on its first line only, so the "every line belongs to a node" check failed the document
        - repaired in the one shared locator: `BlockLocator::inline_value` sends quoted scalars through the existing `yaml_scalar.rs` decoder (folding, blank lines, `''`, escapes, end-of-line `\`); flow collections may close on a later line; new `flow_scalar_across_lines` decodes a flow item that crosses a line break; `scan_value_scalars` (suggestion spans) decodes quoted and block scalars whole
        - span mapping is exact (an escape maps to its escape bytes); the existing decoded-equals-parsed check still guards every projection; tags, anchors, and aliases stay outside the strict grammar
        - sweep: Markdown frontmatter and DMLS use the same locator; no duplicate locator exists; 15 presentations covered, 11 of which failed against the old locator (single-quoted, blank line, `''`, `a`, end-of-line `\`, quoted sequence item, flow mapping and flow sequence across lines, quoted and plain flow items, quoted key with wrapped value); plain continuation and `|-`/`>+`/`|2-` block scalars were already clean and kept as controls
        - R1-F2.7: YAML folds the value to `base@ this`, which the operand grammar reads as `base@this`; it loads and its spans land on `base` and `this`
        - tests that asserted the old rejection were rewritten to use a tagged continuation (still outside the grammar) in `darkmatter/lib/tests/l1/schema_yaml_spellings.rs`, `schemas_source_projection.rs`, and `darkmatter/dmls/tests/l1/schema_yaml_spellings.rs`; the `p14_multiline_inline.md` corpus exception in `shipped_schema_corpus.rs` was removed because it now projects
        - docs: `types.md` gained "Quoted scalars and flow collections across lines"; `diagnostics.md`, `dmls-schema-support.md`, and the darkmatter repo skill (`schema.md`, `compose.md`) updated; stale `source.rs` doc comments corrected
        - decision: widen the strict grammar rather than load schemas with the tolerant grammar, which would also admit tags and comments inside flow collections
        - orchestrator check: the remaining "closed boundary" paragraph in `types.md` now lists only presentations still outside the grammar, so it no longer contradicts the new section
- work completed for 'R1-F2 (multiline quoted object definitions fail source projection)' at 18:53:47
- starting the work on 'R1-F1 (completion disappears in continued flow-sequence operands)' at 18:54:00
        - defect class: editor cursor-context discovery depends on the cursor line containing the directive, so it misses a cursor inside a flow collection or wrapped scalar opened on an earlier line
        - cause: `flow_value_cursor` returned `None` outside a flow mapping, so `composition_value_start` read only the cursor's line
        - repaired: the frame walk moved into `overlay/schema.rs::flow_frames`; new `block_flow_sequence_start` returns the opening `[` of an enclosing flow sequence and `composition_value_start` resolves the cursor there; `authoring_key_path` follows the new `continued_value_anchor`
        - sibling fixed: completion of `...ba` inside a closed wrapped quoted object definition (`continued_scalar_cursor` accepts a quoted scalar before its closing quote)
        - sibling fixed: the tolerant namespace blanked only the cursor line, so a continuation line lost eligibility checks, receiver exclusion, and the `@this` answer; `blank_lines` now blanks from the line where the value starts
        - sibling fixed (predates this feature's continuation work): a parsed `types`-only library offered the receiving type as its own operand; fixed with `names_types_table`
        - clean: hover, go-to-definition, and diagnostic ranges on continuation-line operands already worked once R1-F2 landed; now covered by `object_composition_editor::operands_on_continuation_lines_complete_navigate_and_report`
        - negative controls: `tags: [a,` continuation, continued `types` union, property list, and `$constraints` do not receive operand completion
        - decision: reuse the existing frame walk and `flow_sequence_item_start` instead of making `flow_value_cursor` report key-less sequences, which would change its other callers
        - docs: `dmls-schema-support.md` footnote 8 names the continued-list and wrapped-definition cases; `use-dmls` already promised per-item completion, so no publication was needed
- work completed for 'R1-F1 (completion disappears in continued flow-sequence operands)' at 19:04:01
- starting the work on 'R1-F5 sweep residuals (declaration tables handled with object-only semantics)' at 19:04:30
        - residual 1 reproduced: a types-only document applied a table-level `$constraints` (`minProperties`) and pattern key (`additionalProperties`) to the root, while beside a `$schema` both were silently dropped
        - no doc, test, or shipped schema relied on either form directly in a `types` table
        - fixed: table mode refuses `$constraints` and pattern keys written directly in a `types` table or `schema(mapping)` value with a located grammar error, in every envelope; both still work inside a declaration's own object
        - decision: refuse rather than make `$constraints` a type name, because a `$`-prefixed name cannot be referenced and the value is almost always a misplaced block; pattern keys cannot be names at all
        - added public `parse_schema_table` and `parse_schema_table_with_source`, the table-mode counterparts of the declaration parsers
        - DMLS: `$constraints` and pattern keys are no longer offered as table type names; a `$extends:` directly in a `schema(mapping)` frontmatter value now gets type-definition completion, not operand completion
        - residual 2 reproduced: DMLS located meta-typed frontmatter failures in object mode, so a `schema(mapping)` value produced a bogus `$extends` diagnostic that replaced the real one; fixed by recording per value whether it is a table and using the table parsers
        - docs updated: `schema-documents.md`, `objects-and-named-types.md`, `schema-types.md`, `custom-vocabulary.md`, `diagnostics.md`, `dmls-schema-support.md`, and the simplified-schema repo skill; shipped artifacts unchanged
- work completed for 'R1-F5 sweep residuals (declaration tables handled with object-only semantics)' at 19:18:09
- starting the full verification gates at 19:18:09
        - simplified-schema `just test`: 2483 passed (default build) and 2103 passed (`--no-default-features`), none at or above 5 s; `just lint` clean; `just doctest` 41 passed; root `just check-tier-coverage simplified-schema` stranded (0)
        - darkmatter `just test` (library, CLI, DMLS, Zed CLI): first run 17535/17536, one failure, `lsp_session::meta_schema_hover_covers_pattern_keys_inline_and_standalone`; its fixture placed pattern keys directly in a `types` table, which R1-F5.S3 now refuses; fixture moved to the tagged `$schema` envelope (R1-F5.S5); rerun 17536/17536, none at or above 5 s; `just lint` clean
        - claudine `just test`: 12690/12690, none at or above 5 s; `just lint` clean
        - `cargo check --locked --workspace --all-targets` finishes (covers Messenger and every other reverse dependency)
        - `python3 scripts/shipped_schemas.py` (the root `just shipped-schemas`): "every shipped artifact is current", no generated file changed
        - caller skills: no use-skill transcludes a changed passage, so `just publish-skills` was not needed
        - cross-OS: no path, process, or OS-specific code changed (CRLF projection is covered by L1 tests), so no Windows cross-check was run; CI covers the other environments
- verification gates completed at 19:40:27

### Completion Report

- **outcome: complete**; elapsed about 1 h 24 min (18:16:34 → 19:40:27 UTC)
- parent findings: total 5 = fixed 5 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
        - instance rows: 42 review rows (28 fixed, 14 clean) plus 13 sweep rows (11 fixed, 2 clean)
- verified repairs:
        - R1-F1: `$extends` operands on continuation lines of a flow sequence complete in standalone YAML and frontmatter; closed wrapped quoted definitions complete `...name`; hover, definition, and diagnostics are covered on those lines
        - R1-F2: the shared strict YAML locator projects quoted scalars, flow collections, and flow items that cross line breaks, with exact byte spans
        - R1-F3: `claudine/schemas/sequence-types.yaml` restored; every `include_str!` of it compiles
        - R1-F4: `sequence.yaml` back on the trigger contract; the duplicate registration draft `claudine/schemas/lifecycle-events.yaml` removed; all five failed consumer tests pass
        - R1-F5: `schema(mapping)` reads a declaration table, so `$extends` is an ordinary name in `types` everywhere; `$constraints` and pattern keys written directly in a table are now refused with a located error instead of being applied to the root or dropped
- open findings: none
- author-facing decision recorded in the review's `human-review-items`: restoring the sequence/type split and removing the published lifecycle draft reverse parts of the author's commits `8f33de0d7` and `89ffd8678`
- changed files:
        - R1-F1: `darkmatter/dmls/src/overlay/schema.rs`, `darkmatter/dmls/src/providers/frontmatter.rs`, `darkmatter/dmls/src/providers/frontmatter/named_references.rs`, `darkmatter/dmls/src/providers/frontmatter/composition_tests.rs`, `darkmatter/dmls/tests/l1/object_composition_editor.rs`
        - R1-F2: `simplified-schema/lib/src/simplified/source.rs`, `simplified-schema/lib/tests/l1/{object_composition,located_schema_errors}.rs`, `darkmatter/lib/tests/l1/{schema_yaml_spellings,schemas_source_projection,shipped_schema_corpus}.rs`, `darkmatter/dmls/tests/l1/schema_yaml_spellings.rs`, `simplified-schema/docs/topics/language/types.md`, `.claude/skills/darkmatter/{schema,compose}.md`
        - R1-F3/R1-F4: `claudine/schemas/sequence-types.yaml` (restored), `claudine/schemas/sequence.yaml`, `claudine/schemas/lifecycle-events.yaml` (deleted), `claudine/lib/generated/shipped.json`, `claudine/schemas/README.md`, `claudine/docs/topics/schemas.md`, `simplified-schema/lib/tests/l1/shipped_corpus.rs`, `darkmatter/dmls/tests/l1/claudine_schema_discovery.rs`
        - R1-F5 and sweep: `simplified-schema/lib/src/{format,lib}.rs`, `simplified-schema/lib/src/simplified/mod.rs`, `simplified-schema/lib/src/triggers/matcher.rs`, `simplified-schema/lib/tests/l1/{object_composition,declaration_narrowing}.rs`, `darkmatter/lib/tests/l1/yaml_kind_registry.rs`, `darkmatter/dmls/src/diagnostics/frontmatter.rs`, `darkmatter/dmls/tests/l1/lsp_session.rs`, `simplified-schema/docs/topics/language/{objects-and-named-types,schema-documents,schema-types}.md`, `simplified-schema/docs/topics/{custom-vocabulary,diagnostics}.md`, `.claude/skills/simplified-schema/{SKILL,declaration-narrowing}.md`
        - shared docs: `darkmatter/docs/topics/schemas/dmls-schema-support.md` (R1-F1, R1-F2, R1-F4, R1-F5)
- no commit, no formatter, no terminal or browser window was opened

### Review verification note — 2026-11-22, iteration 2

The broad claim that wrapped quoted definitions complete `...name` was
missing a condition: the new continuation path requires the closing quote
to exist. Both closed quote styles work; leaving either quote open returns
no completion. Folded/literal frontmatter spreads and wrapped scalar or
block-list `$extends` operands also lose composition context. Provider
probes covered 34 cases, and an LSP session returned `[]` instead of
`["base"]` for a folded frontmatter spread (0.064 s). These newly found
siblings are R2-F1 in review-2.md; they do not invalidate verification of
the original R1-F1 flow-list instances. The completion report above records
the repair's state before this subsequent review.

## Implementation of Review Findings #2

> **started at:** 2026-11-22T19:46:26+00:00

- this implementation is attempting to implement _all_ of the review findings found in '/home/build/wt/rusty-biscuit/fix-biscuit-terminal/simplified-schema/features/2026-11-22-object-composition/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- the review's `implemented` flag was confirmed `false` before repairs began
- the review has no `human-in-the-loop` items and no blocked findings, so no finding can be blocked on a ruling
- the review reports R2-F1 as a recurrence of R1-F1; its sweep table (34 rows) is the minimum scope

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R2-F1 | Scalar continuation completion still loses composition context | fixed | Class repaired in the shared reader `decode_scalar_node_prefix` and DMLS `composition_value` / `continued_scalar_cursor`; all 34 rows asserted by `composition_operands_on_scalar_continuation_lines`; sweep rows S1–S8 below |
| R2-F1.1 | Standalone spread, closed multiline single quote → `base` | clean | `composition_operands_on_scalar_continuation_lines` case `standalone closed single quote` (exact labels and replaced range) |
| R2-F1.2 | Frontmatter spread, closed multiline single quote → `base` | clean | `composition_operands_on_scalar_continuation_lines` case `frontmatter closed single quote` (exact labels and replaced range) |
| R2-F1.3 | Standalone spread, closed multiline double quote → `base` | clean | `composition_operands_on_scalar_continuation_lines` case `standalone closed double quote` (exact labels and replaced range) |
| R2-F1.4 | Frontmatter spread, closed multiline double quote → `base` | clean | `composition_operands_on_scalar_continuation_lines` case `frontmatter closed double quote` (exact labels and replaced range) |
| R2-F1.5 | Standalone spread, open multiline single quote → `base` | fixed | `composition_operands_on_scalar_continuation_lines` case `standalone open single quote` (exact labels and replaced range); CRLF in `composition_operands_on_crlf_continuation_lines` |
| R2-F1.6 | Frontmatter spread, open multiline single quote → `base` | fixed | `composition_operands_on_scalar_continuation_lines` case `frontmatter open single quote` (exact labels and replaced range); CRLF in `composition_operands_on_crlf_continuation_lines`; served over its closed last-good buffer |
| R2-F1.7 | Standalone spread, open multiline double quote → `base` | fixed | `composition_operands_on_scalar_continuation_lines` case `standalone open double quote` (exact labels and replaced range) |
| R2-F1.8 | Frontmatter spread, open multiline double quote → `base` | fixed | `composition_operands_on_scalar_continuation_lines` case `frontmatter open double quote` (exact labels and replaced range); served over its closed last-good buffer |
| R2-F1.9 | Standalone spread, folded `>-` → `base` | clean | `composition_operands_on_scalar_continuation_lines` case `standalone folded` (exact labels and replaced range); CRLF in `composition_operands_on_crlf_continuation_lines` |
| R2-F1.10 | Frontmatter spread, folded `>-` → `base` (LSP session too) | fixed | `composition_operands_on_scalar_continuation_lines` case `frontmatter folded` (exact labels and replaced range); CRLF in `composition_operands_on_crlf_continuation_lines`; LSP session `object_composition_editor::operands_in_block_scalars_and_wrapped_quotes_complete` (folded spread) |
| R2-F1.11 | Standalone spread, literal `|-` → `base` | clean | `composition_operands_on_scalar_continuation_lines` case `standalone literal` (exact labels and replaced range) |
| R2-F1.12 | Frontmatter spread, literal `|-` → `base` | fixed | `composition_operands_on_scalar_continuation_lines` case `frontmatter literal` (exact labels and replaced range) |
| R2-F1.13 | Standalone scalar `$extends` plain → `base` | clean | `composition_operands_on_scalar_continuation_lines` case `standalone $extends plain` (exact labels and replaced range); CRLF in `composition_operands_on_crlf_continuation_lines` |
| R2-F1.14 | Frontmatter scalar `$extends` plain → `base` | clean | `composition_operands_on_scalar_continuation_lines` case `frontmatter $extends plain` (exact labels and replaced range); CRLF in `composition_operands_on_crlf_continuation_lines` |
| R2-F1.15 | Standalone scalar `$extends` single quote → `base` | clean | `composition_operands_on_scalar_continuation_lines` case `standalone $extends single quote` (exact labels and replaced range) |
| R2-F1.16 | Frontmatter scalar `$extends` single quote → `base` | clean | `composition_operands_on_scalar_continuation_lines` case `frontmatter $extends single quote` (exact labels and replaced range) |
| R2-F1.17 | Standalone scalar `$extends` double quote → `base` | clean | `composition_operands_on_scalar_continuation_lines` case `standalone $extends double quote` (exact labels and replaced range) |
| R2-F1.18 | Frontmatter scalar `$extends` double quote → `base` | clean | `composition_operands_on_scalar_continuation_lines` case `frontmatter $extends double quote` (exact labels and replaced range) |
| R2-F1.19 | Standalone scalar `$extends` folded `>-` → only `base` (eligibility) | fixed | `composition_operands_on_scalar_continuation_lines` case `standalone $extends folded` (exact labels and replaced range); LSP session `object_composition_editor::operands_in_block_scalars_and_wrapped_quotes_complete` (folded standalone `$extends`) |
| R2-F1.20 | Frontmatter scalar `$extends` folded `>-` → `base` | fixed | `composition_operands_on_scalar_continuation_lines` case `frontmatter $extends folded` (exact labels and replaced range) |
| R2-F1.21 | Standalone scalar `$extends` literal `|-` → only `base` (eligibility) | fixed | `composition_operands_on_scalar_continuation_lines` case `standalone $extends literal` (exact labels and replaced range); CRLF in `composition_operands_on_crlf_continuation_lines` |
| R2-F1.22 | Frontmatter scalar `$extends` literal `|-` → `base` | fixed | `composition_operands_on_scalar_continuation_lines` case `frontmatter $extends literal` (exact labels and replaced range); CRLF in `composition_operands_on_crlf_continuation_lines` |
| R2-F1.23 | Standalone scalar `$extends` wrapped single-quoted `base@`/`th` → `this` | fixed | `composition_operands_on_scalar_continuation_lines` case `standalone $extends wrapped single quote` (exact labels and replaced range) |
| R2-F1.24 | Frontmatter scalar `$extends` wrapped single quote → `this` | fixed | `composition_operands_on_scalar_continuation_lines` case `frontmatter $extends wrapped single quote` (exact labels and replaced range); LSP session `object_composition_editor::operands_in_block_scalars_and_wrapped_quotes_complete` (wrapped scalar `$extends`) |
| R2-F1.25 | Standalone scalar `$extends` wrapped double-quoted `base@`/`th` → `this` | fixed | `composition_operands_on_scalar_continuation_lines` case `standalone $extends wrapped double quote` (exact labels and replaced range) |
| R2-F1.26 | Frontmatter scalar `$extends` wrapped double quote → `this` | fixed | `composition_operands_on_scalar_continuation_lines` case `frontmatter $extends wrapped double quote` (exact labels and replaced range) |
| R2-F1.27 | Standalone flow-list item wrapped single quote → `this` | clean | `composition_operands_on_scalar_continuation_lines` case `standalone flow item wrapped quote` (exact labels and replaced range) |
| R2-F1.28 | Frontmatter flow-list item wrapped single quote → `this` | clean | `composition_operands_on_scalar_continuation_lines` case `frontmatter flow item wrapped quote` (exact labels and replaced range) |
| R2-F1.29 | Standalone block-list item wrapped single quote → `this` | fixed | `composition_operands_on_scalar_continuation_lines` case `standalone block item wrapped quote` (exact labels and replaced range); CRLF in `composition_operands_on_crlf_continuation_lines` |
| R2-F1.30 | Frontmatter block-list item wrapped single quote → `this` | fixed | `composition_operands_on_scalar_continuation_lines` case `frontmatter block item wrapped quote` (exact labels and replaced range); CRLF in `composition_operands_on_crlf_continuation_lines`; LSP session `object_composition_editor::operands_in_block_scalars_and_wrapped_quotes_complete` (wrapped block item) |
| R2-F1.31 | Standalone flow-list unquoted continuation → `base` | clean | `composition_operands_on_scalar_continuation_lines` case `standalone flow item continued` (exact labels and replaced range) |
| R2-F1.32 | Frontmatter flow-list unquoted continuation → `base` | clean | `composition_operands_on_scalar_continuation_lines` case `frontmatter flow item continued` (exact labels and replaced range) |
| R2-F1.33 | Standalone block-list `- b` → `base` | clean | `composition_operands_on_scalar_continuation_lines` case `standalone block item` (exact labels and replaced range) |
| R2-F1.34 | Frontmatter block-list `- b` → `base` | clean | `composition_operands_on_scalar_continuation_lines` case `frontmatter block item` (exact labels and replaced range) |
| R2-F1.S1 | Scalar `$extends` reference in a quote still open at the cursor (single and double, `'base@` / `th`), standalone and frontmatter → `this` | fixed | `composition_operands_in_further_scalar_presentations` cases `$extends open single quote`, `$extends open double quote`; shares the repaired route of rows 23–26 (not measured separately before the repair) |
| R2-F1.S2 | Block-list `$extends` item in a quote still open at the cursor → `this` | fixed | `composition_operands_in_further_scalar_presentations` case `block item open quote`; shares the route of rows 29–30 (not measured separately before the repair) |
| R2-F1.S3 | Further presentations: wrapped plain `base@` / `th` → `this`; folded `>+` and `>`, literal `|2-` (indentation indicator), `$extends: |+`, a spread on the first block-scalar line, and a folded block-list item → `base` | fixed | `composition_operands_in_further_scalar_presentations` (exact labels and ranges, standalone and frontmatter); not measured separately before the repair |
| R2-F1.S4 | Empty operand on a continuation line (folded `$extends`, open-quote reference half, spread in a folded definition, spread in an open quote) → every eligible object or `this`, receiver excluded | fixed | `composition_empty_operand_on_a_continuation_line_offers_only_eligible_objects`; frontmatter buffers that do not parse are served over a last-good tree |
| R2-F1.S5 | Recovery does not over-reach: frontmatter data `$extends`/`note` folded values, a `types` entry named `$extends` with a folded value, text past a wrapped reference's closing quote, a sibling key after a block scalar | clean | `composition_continuation_recovery_keeps_other_values_ordinary` |
| R2-F1.S6 | Hover and go-to-definition on operand names on continuation lines (folded and literal spreads, folded `$extends`, wrapped quoted `$extends` import, wrapped block item), standalone and frontmatter: they read the library source map, not line-only discovery | clean | `composition_operand_navigation_on_continuation_lines` |
| R2-F1.S7 | Operand diagnostics on continuation lines (folded and literal spreads, folded `$extends`, wrapped block item) are ranged on the operand | clean | `composition_operand_diagnostics_on_continuation_lines` |
| R2-F1.S8 | Regression found during verification: the new reader first treated an empty holder value (`entry:` followed by a sequence) as the start of a plain scalar, so a sibling line offered constraints | fixed | `schema_yaml_spellings::a_line_the_plain_scalar_does_not_reach_offers_none_of_its_completions` (sequence-sibling case) and `scalar_node_prefix_rejects_a_cursor_outside_the_scalar` |

### Progress

- starting the work on 'R2-F1: Scalar continuation completion still loses composition context' at 19:47:07
        - defect class: editor operand discovery reads only the cursor's own line, so a cursor on a continuation line of a scalar (quoted open or closed, folded, or literal) loses the owning key and its composition context; fallback routing then treats the operand as an ordinary type definition
        - delegated to one foreground rust-developer subagent, which logs its own progress below
        - reproduced all 34 rows with the table test `composition_operands_on_scalar_continuation_lines` (`darkmatter/dmls/src/providers/frontmatter/composition_tests.rs`), which collects every mismatch: exactly the review's 16 failing rows fail (5–8, 10, 12, 19–26, 29, 30), with rows 19 and 21 offering `boolean`, `boolean[]`, `boolish`, `boolish[]`, `base`; the 18 clean rows pass
        - added the shared `simplified_schema::decode_scalar_node_prefix` (`simplified-schema/lib/src/simplified/yaml_scalar.rs`): the part of a scalar node before a cursor in every presentation `decode_scalar_node` models, with a quoted scalar read only up to the cursor so an unclosed quote still decodes; `decode_flow_quoted` gained a tolerant mode for it; unit tests `scalar_node_prefix_reads_every_presentation_up_to_the_cursor` (LF and CRLF, nine presentations) and `scalar_node_prefix_rejects_a_cursor_outside_the_scalar`
        - DMLS routing (`darkmatter/dmls/src/providers/frontmatter.rs`): `composition_value_start` became `composition_value`, which recovers a scalar or block-item `$extends` continued onto the cursor's line through `continued_line_holder` and the new decoder and reads the operand with `locate_composition_operand_cursor_in_scalar`; it runs before the standalone-model fallback, so rows 19 and 21 no longer reach ordinary type keywords; `continued_scalar_cursor` now uses the same decoder, so spreads continue through open quotes and literal/folded block scalars
        - frontmatter that does not parse has no authoring state on a cold start (the documented baseline in `enum_item_tests.rs`); an editor reaches an open quote from a last-good tree, so the frontmatter open-quote rows (6, 8) are served after their closed last-good buffer, while the standalone rows stay cold
        - all 34 rows pass in `composition_operands_on_scalar_continuation_lines`; `composition_operands_on_crlf_continuation_lines` repeats an open quote, a folded spread, a plain and a literal `$extends`, and a wrapped block item with CRLF
        - sweep beyond the review's table (rows S1–S8): open-quote references, further chomping and indentation indicators, block items holding block scalars, wrapped plain references, empty operands, and negative controls; hover, go-to-definition, and operand diagnostics already read the library source map for every presentation and needed no change
        - verification found one regression of the new reader (S8): an empty holder value started a plain scalar on the next line; `decode_scalar_node_prefix` now treats an empty value as `decode_plain_scalar_node` does
        - LSP session controls added in `darkmatter/dmls/tests/l1/object_composition_editor.rs` (`operands_in_block_scalars_and_wrapped_quotes_complete`): folded frontmatter spread → `base`, wrapped scalar `$extends` → `this`, wrapped block item → `this`, folded standalone `$extends` → only `base`
        - docs: `darkmatter/docs/topics/schemas/dmls-schema-support.md` (the "only a plain scalar continues" passage had drifted; now lists every continuing presentation with an example and the last-good reading of unparseable frontmatter; the operand completion row and the public-primitives list), `simplified-schema/docs/topics/diagnostics.md` (new helper), `.claude/skills/darkmatter/schema.md`, and the `*_in_scalar` docs in `simplified-schema/lib/src/simplified/cursor.rs`
        - verification: scoped DMLS run 458/458, DMLS `--lib` 959/959, DMLS schema L1 315/315, simplified-schema decoder/cursor/source filter 407/407, Darkmatter projection consumers 60/60, clippy `-D warnings` clean for `darkmatter-lsp` and `simplified-schema`; no test reached 5 s
- work completed for 'R2-F1: Scalar continuation completion still loses composition context' at 20:10:48
        - orchestrator reconciliation: inspected the shared decoder diff (existing callers pass `tolerant: false`, so their behavior is unchanged) and the row-driven tests (exact labels, exact replaced byte ranges, extra results fail the assertion)
        - boundary recorded for rows 6 and 8: frontmatter that does not parse is read over the last frontmatter that did (existing, documented behavior, `darkmatter/docs/topics/schemas/dmls-schema-support.md`), which is how an open editor reaches an unclosed quote; a cold open of such a file offers no authoring completion of any kind, composition or otherwise, and this cycle does not change that

### Completion Report

- **outcome: complete**; elapsed about 24 min (19:46:26 → 20:10:48 UTC)
- parent findings: total 1 = fixed 1 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
        - instance rows: 34 review rows (16 fixed, 18 clean, each now asserted) plus 8 sweep rows (S1–S4 and S8 fixed, S5–S7 clean)
- verified repairs:
        - R2-F1: a composition operand on a continuation line of a plain, single/double-quoted (closed or still open at the cursor), folded, or literal scalar is recovered through the shared `simplified_schema::decode_scalar_node_prefix`; `$extends` scalars, block items, and flow items route through the composition operand reader before the standalone-model fallback, so only eligible objects (or `this`) are offered; spreads beginning the tested continuation lines complete `base`; hover, definition, and diagnostics were checked on the same presentations (clean). This does not cover a continuation line containing an explicit property before the spread; see the iteration 3 verification note below
- open findings: none
- no `human-review-items` added; no decision met the author-time threshold
- changed files (all R2-F1):
        - `simplified-schema/lib/src/simplified/yaml_scalar.rs`, `simplified-schema/lib/src/simplified/mod.rs`, `simplified-schema/lib/src/lib.rs`, `simplified-schema/lib/src/simplified/cursor.rs`
        - `darkmatter/dmls/src/providers/frontmatter.rs`, `darkmatter/dmls/src/providers/frontmatter/composition_tests.rs`, `darkmatter/dmls/tests/l1/object_composition_editor.rs`
        - docs: `darkmatter/docs/topics/schemas/dmls-schema-support.md`, `simplified-schema/docs/topics/diagnostics.md`, `.claude/skills/darkmatter/schema.md`
- final verification (run by the orchestrator after all changes, host load about 4 on 16 cores):
        - simplified-schema `just test`: 2485/2485 (default build) and 2105/2105 (`--no-default-features`); `just lint` clean; `just doctest` 41 passed; `just check-tier-coverage simplified-schema` stranded (0)
        - darkmatter `just test` (library, CLI, DMLS, Zed): 17544/17544, "none at or above 5.0s"; `just lint` clean, including the zed-dmls wasm check
        - `just slow-tests fresh`: fresh
        - `cargo check --locked -p claudine -p claudine-cli --tests`: succeeds (downstream consumer of the changed library)
        - cross-OS check not run: the change is string and cursor handling with CRLF covered and no path or OS-specific code
- no commit, no formatter, no terminal or browser window was opened


### Review verification note — 2026-11-22, iteration 3

The completion report's broad statement that spreads inside continuing
scalars complete `base` omitted a condition. The 34 review rows and eight
repair sweep rows pass, but a continuation line containing an explicit
property before the spread (`next: number, ...b`) can still be read as a
YAML mapping entry. In frontmatter this offers `boolean`, `boolean[]`,
`boolish`, and `boolish[]` alongside `base`; an open standalone quote can
lose `base` entirely. A `...base@th` reference-half completion in folded
frontmatter returns no items instead of `this`. The report above is now
limited to the cases actually verified by that repair.

Verified with temporary provider probes over 32 scalar/host presentations,
four further frontmatter owner cases, and a `textDocument/completion`
session assertion that returned the five labels instead of only `base`
(0.047 seconds, nextest run
`b65dd68b-5c61-4859-b74c-7d7c83fd6613`). Hover, definition, and located
operand-diagnostic checks passed for eight closed-scalar cases in run
`7557bbb0-0d42-402b-b83b-0681590aa46e`. All temporary probe source additions
were removed. This is the newly found sibling R3-F1 in review-3.md, not a
failure of any required instance previously listed under R2-F1.

## Implementation of Review Findings #3

> **started at:** 2026-11-22T20:17:09+00:00

- this implementation is attempting to implement _all_ of the review findings found in '/home/build/wt/rusty-biscuit/fix-biscuit-terminal/simplified-schema/features/2026-11-22-object-composition/review-3.md'
- this is iteration 3 of the review-to-implement cycle 
- the review's `implemented` flag was confirmed `false` before repairs began
- the review has no `human-in-the-loop` items and no blocked findings, so no finding can be blocked on a ruling
- the review reports R3-F1 as a recurrence of R1-F1 and R2-F1; its 41-row sweep table and the recurrence list (per-line mapping reader, backward scalar-holder scan, receiving-object scope, tolerant namespace anchor, composition cursor, eligibility filter) are the minimum scope

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R3-F1 | Explicit properties on scalar continuation lines hide spread completion context | fixed | Class repaired in DMLS `scalar_holders` (shared decoder decides scalar ownership before any per-line `key:` reading) and its consumers `line_cursor`, `composition_value`, `continued_value_anchor`, `authoring_key_path`, `schema_completion`; all 41 review rows and sweep rows S1–S2 asserted; darkmatter `just test` 17552/17552 |
| R3-F1.1 | Standalone spread, closed single quote, continuation `next: number, ...b` → `base` (clean in review) | clean | `composition_operands_after_an_explicit_property_on_a_continuation_line` case `standalone closed single quote` (exact labels and edit range); passed before the repair too |
| R3-F1.2 | Frontmatter spread, same closed single quote → `base` | fixed | `composition_operands_after_an_explicit_property_on_a_continuation_line` case `frontmatter closed single quote` (exact labels and edit range); failed before the repair |
| R3-F1.3 | Standalone spread, closed double quote → `base` (clean in review) | clean | `composition_operands_after_an_explicit_property_on_a_continuation_line` case `standalone closed double quote` (exact labels and edit range); passed before the repair too |
| R3-F1.4 | Frontmatter spread, closed double quote → `base` | fixed | `composition_operands_after_an_explicit_property_on_a_continuation_line` case `frontmatter closed double quote` (exact labels and edit range); failed before the repair |
| R3-F1.5 | Standalone spread, open single quote → `base` | fixed | `composition_operands_after_an_explicit_property_on_a_continuation_line` case `standalone open single quote` (exact labels and edit range); failed before the repair (lost `base`) |
| R3-F1.6 | Frontmatter spread, open single quote over last-good tree → `base` | fixed | `composition_operands_after_an_explicit_property_on_a_continuation_line` case `frontmatter open single quote` (exact labels and edit range), served over its closed last-good buffer; CRLF repeat |
| R3-F1.7 | Standalone spread, open double quote → `base` | fixed | `composition_operands_after_an_explicit_property_on_a_continuation_line` case `standalone open double quote` (exact labels and edit range); failed before the repair (lost `base`) |
| R3-F1.8 | Frontmatter spread, open double quote over last-good tree → `base` | fixed | `composition_operands_after_an_explicit_property_on_a_continuation_line` case `frontmatter open double quote` (exact labels and edit range), served over its closed last-good buffer |
| R3-F1.9 | Standalone spread, folded `>-` → `base` (clean in review) | clean | `composition_operands_after_an_explicit_property_on_a_continuation_line` case `standalone folded` (exact labels and edit range); passed before the repair too |
| R3-F1.10 | Frontmatter spread, folded `>-` → `base` | fixed | `composition_operands_after_an_explicit_property_on_a_continuation_line` case `frontmatter folded` (exact labels and edit range); CRLF in `composition_operands_after_an_explicit_property_on_crlf_continuation_lines`; LSP session `object_composition_editor::operands_in_block_scalars_and_wrapped_quotes_complete` case `folded spread after a property` → exactly `base` |
| R3-F1.11 | Standalone spread, literal `|-` → `base` (clean in review) | clean | `composition_operands_after_an_explicit_property_on_a_continuation_line` case `standalone literal` (exact labels and edit range); passed before the repair too |
| R3-F1.12 | Frontmatter spread, literal `|-` → `base` | fixed | `composition_operands_after_an_explicit_property_on_a_continuation_line` case `frontmatter literal` (exact labels and edit range); failed before the repair |
| R3-F1.13 | Standalone folded, `next: number,` on an earlier line then more-indented `...b` → `base` (clean in review) | clean | `composition_operands_after_an_explicit_property_on_a_continuation_line` case `standalone folded, property on an earlier line` (exact labels and edit range); passed before the repair too |
| R3-F1.14 | Frontmatter, same earlier property line → `base` | fixed | `composition_operands_after_an_explicit_property_on_a_continuation_line` case `frontmatter folded, property on an earlier line` (exact labels and edit range); CRLF repeat; failed before the repair |
| R3-F1.15 | Standalone closed single quote, earlier property line → `base` (clean in review) | clean | `composition_operands_after_an_explicit_property_on_a_continuation_line` case `standalone closed single quote, property on an earlier line` (exact labels and edit range); passed before the repair too |
| R3-F1.16 | Frontmatter, same earlier property line in single quote → `base` | fixed | `composition_operands_after_an_explicit_property_on_a_continuation_line` case `frontmatter closed single quote, property on an earlier line` (exact labels and edit range); failed before the repair |
| R3-F1.17 | Standalone array-item spread `{ at: string,` / `next: number, ...b }[]` → `base` (clean in review) | clean | `composition_operands_after_an_explicit_property_on_a_continuation_line` case `standalone folded array item` (exact labels and edit range); passed before the repair too |
| R3-F1.18 | Frontmatter array-item spread → `base` | fixed | `composition_operands_after_an_explicit_property_on_a_continuation_line` case `frontmatter folded array item` (exact labels and edit range); failed before the repair |
| R3-F1.19 | Standalone union-arm spread `… ...b } \| string` → `base` (clean in review) | clean | `composition_operands_after_an_explicit_property_on_a_continuation_line` case `standalone folded union arm` (exact labels and edit range); passed before the repair too |
| R3-F1.20 | Frontmatter union-arm spread → `base` | fixed | `composition_operands_after_an_explicit_property_on_a_continuation_line` case `frontmatter folded union arm` (exact labels and edit range); failed before the repair |
| R3-F1.21 | Standalone folded `next: number, ...base@th` → `this` (clean in review) | clean | `composition_operands_after_an_explicit_property_on_a_continuation_line` case `standalone folded reference half` (exact labels and edit range); passed before the repair too |
| R3-F1.22 | Frontmatter folded `...base@th` → `this` | fixed | `composition_operands_after_an_explicit_property_on_a_continuation_line` case `frontmatter folded reference half` (exact labels and edit range); failed before the repair |
| R3-F1.23 | Standalone one-line single quote control → `base` | clean | `composition_operands_after_an_explicit_property_on_a_continuation_line` case `standalone one-line single quote` (exact labels and edit range) |
| R3-F1.24 | Frontmatter one-line single quote control → `base` | clean | `composition_operands_after_an_explicit_property_on_a_continuation_line` case `frontmatter one-line single quote` (exact labels and edit range) |
| R3-F1.25 | Standalone one-line double quote control → `base` | clean | `composition_operands_after_an_explicit_property_on_a_continuation_line` case `standalone one-line double quote` (exact labels and edit range) |
| R3-F1.26 | Frontmatter one-line double quote control → `base` | clean | `composition_operands_after_an_explicit_property_on_a_continuation_line` case `frontmatter one-line double quote` (exact labels and edit range) |
| R3-F1.27 | Standalone whole object on first folded content line control → `base` | clean | `composition_operands_after_an_explicit_property_on_a_continuation_line` case `standalone first folded line` (exact labels and edit range) |
| R3-F1.28 | Frontmatter whole object on first folded content line control → `base` | clean | `composition_operands_after_an_explicit_property_on_a_continuation_line` case `frontmatter first folded line` (exact labels and edit range) |
| R3-F1.29 | Standalone whole object on first literal content line control → `base` | clean | `composition_operands_after_an_explicit_property_on_a_continuation_line` case `standalone first literal line` (exact labels and edit range) |
| R3-F1.30 | Frontmatter whole object on first literal content line control → `base` | clean | `composition_operands_after_an_explicit_property_on_a_continuation_line` case `frontmatter first literal line` (exact labels and edit range) |
| R3-F1.31 | Standalone nested-object spread `{ nested: { at: string,` / `next: number, ...b } }` → `base` (clean in review) | clean | `composition_operands_after_an_explicit_property_on_a_continuation_line` case `standalone folded nested object` (exact labels and edit range); passed before the repair too |
| R3-F1.32 | Frontmatter nested-object spread → `base` | fixed | `composition_operands_after_an_explicit_property_on_a_continuation_line` case `frontmatter folded nested object` (exact labels and edit range); failed before the repair |
| R3-F1.33 | Frontmatter `type-definition` value `shape: >-` with local `base` in `$schema` → `base` | fixed | `composition_owner_after_an_explicit_property_on_a_continuation_line` case `type-definition value` → exactly `base`; failed before the repair (extra primitive suggestions) |
| R3-F1.34 | Object inside frontmatter `schema` value `shape.event: >-` → `base` | fixed | `composition_owner_after_an_explicit_property_on_a_continuation_line` case `object in a schema value` → exactly `base`; failed before the repair (extra primitive suggestions) |
| R3-F1.35 | Declaration inside frontmatter `schema(mapping)` value → `base` | fixed | `composition_owner_after_an_explicit_property_on_a_continuation_line` case `declaration in a schema(mapping) value` → exactly `base`; failed before the repair (extra primitive suggestions) |
| R3-F1.36 | Implicit root-union arm scope, folded receiving `event` → only arm-local `base` | fixed | `composition_owner_after_an_explicit_property_on_a_continuation_line` case `root-union arm` → exactly `base`; failed before the repair (extra primitives, and the other arm's `boxed` leaked) |
| R3-F1.37 | `$extends` scalar operand route/eligibility (plain, quoted, wrapped, folded, literal; both hosts) → eligible names / reference completions | clean | existing scalar `$extends` matrices in `composition_operands_on_scalar_continuation_lines` pass unchanged; explicit-property variants in `composition_extends_after_a_property_holding_a_key_like_line` |
| R3-F1.38 | `$extends` sequence operand route/eligibility (continued flow, wrapped quoted flow/block, block items; both hosts) | clean | existing continued-flow/wrapped/block item matrices pass unchanged; `composition_extends_after_a_property_holding_a_key_like_line` |
| R3-F1.39 | Operand hover and go-to-definition after the explicit property (8 cases) → local declaration | clean | `composition_operand_navigation_after_an_explicit_property` (8 cases: four closed presentations × standalone/frontmatter) reach the local `base` declaration |
| R3-F1.40 | Operand diagnostic projection with primitive `label` (8 cases) → located invalid-target error | clean | `composition_operand_diagnostics_after_an_explicit_property` (8 cases) — diagnostic range is exactly `label`, message explains ineligibility |
| R3-F1.41 | Non-composition contexts and scalar boundaries (data values, table key `$extends`, text after closing quote, sibling after block scalar, sequence sibling after empty holder) → ordinary completion | clean | existing `composition_continuation_recovery_keeps_other_values_ordinary` plus `composition_explicit_property_recovery_keeps_real_siblings_ordinary` (real `next:` sibling after a folded and after a closed-quote definition, sequence sibling after empty holder, data value, `types` entry named `$extends`, text past a closing quote) |
| R3-F1.S1 | Sweep: ordinary (non-authoring) frontmatter completion also read a `key:`-looking line inside a folded/quoted data scalar as a key (e.g. `next: tr` offered `true`) → no value completion on a line a scalar owns | fixed | `key_like_line_on_a_data_scalar_continuation_offers_no_value_completion` (folded, single/double quote, folded `- ` line; real entry, sibling after the folded value, and real item still complete); failed before the guard |
| R3-F1.S2 | Sweep: a `#` line inside a folded or double-quoted definition before `next: number, ...b` ended the holder scan, though inside a quoted/block scalar it is text → `base` | fixed | `composition_operands_after_an_explicit_property_on_a_continuation_line` cases `folded, # line before the property`, `closed double quote, # line before the property` (both hosts); failed before, including closed standalone; a `#` line still ends a plain scalar (decided by the shared decoder) |

### Progress

- starting the work on 'R3-F1: Explicit properties on scalar continuation lines hide spread completion context' at 20:17:48
        - defect class: line-based editor context discovery reads a `key:`-looking fragment as a YAML mapping entry without first asking whether an earlier scalar already owns that line, so text inside a continuing quoted or block scalar is misread as a new node (both on the cursor's line and on intermediate lines the backward holder scan passes)
        - delegated to one foreground rust-developer subagent, which logs its own progress below
        - reproduced before any source change with the new table test `composition_operands_after_an_explicit_property_on_a_continuation_line` (rows 1–32 as `EXPLICIT_PROPERTY_OPERANDS`, exact labels and replaced byte ranges, open-quote frontmatter rows served over their closed last-good buffer): exactly the review's failing rows failed (2, 4, 5, 6, 7, 8, 10, 12, 14, 16, 18, 20, 22, 32; standalone open quotes 5 and 7 offered only the four primitives, frontmatter rows offered the five extra-primitive labels, row 22 offered nothing); the 18 clean/control rows passed; `composition_owner_after_an_explicit_property_on_a_continuation_line` (rows 33–36) failed in all four owners, the root-union arm also leaking the other arm's `boxed`
        - repair, `darkmatter/dmls/src/providers/frontmatter.rs`: the backward holder scan `continued_scalar_holder` became `scalar_holders`, which walks every earlier candidate line (indented less than the last, carrying `key:` or `- `) up to the first column-0 line and asks the shared `simplified_schema::decode_scalar_node_prefix` whether its value is a scalar reaching the cursor; the outermost such candidate is the owner, so `next: number,` on the cursor's line or on a line passed on the way is scalar content; `nearest` keeps the previous fallback (nearest candidate, only for a line without its own indicator and with no comment line between) so behavior where no scalar reaches is unchanged; `continued_line_holder` returns the owner, else that fallback
        - `line_cursor` now asks `continued_line_holder` before `value_cursor` (the per-line mapping reader), so an owned `key:`/`- ` line routes to `continued_scalar_cursor`; `continued_scalar_cursor` takes the chosen holder instead of rescanning; `composition_value` (composition cursor), `continued_value_anchor` (namespace blanking in `named_references::tolerant_namespace`, and `authoring_key_path` → `receiving_object_path`, the receiving-object scope) inherit the owner through `continued_line_holder`, which repairs the standalone open-quote rows that lost `base` and the frontmatter eligibility/scope rows; no simplified-schema library change was needed
        - comment lines: the old scan stopped at any `#` line; a `#` line inside a quoted or block scalar is its text, so the scan now continues and only the decoder decides (a comment still ends a plain scalar, which `decode_multiline_plain` already enforces; the `comment-line` case of `a_line_the_plain_scalar_does_not_reach_offers_none_of_its_completions` still passes); new sibling rows S2 below
        - sibling sweep: ordinary frontmatter completion `schema_completion` read the cursor's line with `value_cursor` and offered `outer.next`'s `true` for `next: tr` inside a folded or quoted data value (new sibling S1, reproduced first); it now returns nothing on a line a scalar owns; checked and clean: `block_composition_value_start` (reached only after the owner check), `flow_cursor`/`enclosing_path` (read structural ancestry from a real key line or the AST), `enclosing_path_by_indent` (only reached after routing), `named_references::block_arm_index` (column-0 and item-column lines cannot be quoted/block scalar content), `overlay/schema.rs` top-level envelope reader (skips indented lines, tracks top-level quote state), frontmatter hover/definition and `diagnostics/frontmatter.rs` (AST and library source maps, no per-line key reader)
        - tests added in `darkmatter/dmls/src/providers/frontmatter/composition_tests.rs`: `composition_operands_after_an_explicit_property_on_a_continuation_line` (rows 1–32, plus S2 rows `folded, # line before the property` and `closed double quote, # line before the property`), `composition_operands_after_an_explicit_property_on_crlf_continuation_lines` (open single quote, folded, earlier-line property with CRLF), `composition_owner_after_an_explicit_property_on_a_continuation_line` (rows 33–36), `composition_operand_navigation_after_an_explicit_property` (row 39, 8 cases), `composition_operand_diagnostics_after_an_explicit_property` (row 40, 8 cases, range exactly `label`), `composition_extends_after_a_property_holding_a_key_like_line` (row 37/38 explicit-property variants: `$extends` plain, folded, wrapped quote, and block item after a sibling property whose block/quoted scalar holds `next: number`), `composition_explicit_property_recovery_keeps_real_siblings_ordinary` (row 41 plus explicit variants: real `next:` sibling after a folded and a closed-quote definition, sequence sibling after an empty holder, data value, `types` entry `$extends`, text past a closing quote), `key_like_line_on_a_data_scalar_continuation_offers_no_value_completion` (S1: folded, single, double quote `key:` lines and a folded `- ` line offer nothing; a real entry, a sibling after the folded value, and a real item still offer `true`); LSP session case `folded spread after a property` in `darkmatter/dmls/tests/l1/object_composition_editor.rs::operands_in_block_scalars_and_wrapped_quotes_complete` asserts exactly `base` and its edit range
        - rows 37–41's existing matrices (`composition_operands_on_scalar_continuation_lines`, `composition_operands_in_further_scalar_presentations`, `composition_extends_*`, `composition_operand_navigation_on_continuation_lines`, `composition_operand_diagnostics_on_continuation_lines`, `composition_continuation_recovery_keeps_other_values_ordinary`) pass unchanged; the new navigation, diagnostic, `$extends`-after-property, and negative-control tests also passed against the pre-fix source (clean rows), the S2 rows failed there in both hosts
        - comments and docs: `line_cursor`, `continued_scalar_cursor`, `continued_value_anchor`, `continued_line_holder` docs rewritten, `ScalarHolders`/`scalar_holders` documented (the old "nearest earlier line ... carrying `key:`" and "a comment line ends it" contracts had drifted); `darkmatter/docs/topics/schemas/dmls-schema-support.md` continuation passage now says a comment line ends only a plain scalar and that a `key:`, `- `, or `#` line inside a quoted or block scalar is its text, and its composition example puts `next: number,` before `...b`; `.claude/skills/darkmatter/schema.md` records `scalar_holders` and its four consumers
        - performance: one backward line scan per reader (at most one decode per indentation level, stopping at the first column-0 line); `line_cursor` on a holder line rescans only from that earlier line; the 18-row table runs in about 0.17 s
        - verification: `cargo nextest run --locked -p darkmatter-lsp --lib --test l1 -E 'test(composition) | test(schema_yaml_spellings) | test(claudine_schema_discovery) | test(continu) | test(frontmatter) | test(key_like_line)'` 371/371 (run `b1d6936e-de58-4777-9994-aab68e06c76b`); wider DMLS check `cargo nextest run --locked -p darkmatter-lsp --lib --test l1` 2104/2104, 6 skipped (run `b1016ba7-5918-48cb-874c-f33b762b3ee2`), no test reported slow; `cargo clippy --locked -p darkmatter-lsp --all-targets -- -D warnings` clean; simplified-schema untouched, so its filters and clippy were not rerun; no formatter run, no commit
- work completed for 'R3-F1: Explicit properties on scalar continuation lines hide spread completion context' at 20:32:28
        - orchestrator reconciliation: read the combined `frontmatter.rs` diff; `scalar_holders` makes one backward scan with at most one decode per indentation level, and `nearest` preserves the earlier fallback where no scalar reaches the cursor; the meta-schema route still runs before the ordinary `schema_completion`, which now gives nothing on a line a scalar owns (S1)
        - checked the new tests: exact labels and exact edit ranges through `operand_case_mismatches` / `operand_mismatch`; extra results fail the assertion; all tests are in L1 targets (`--lib` unit tests and `tests/l1`)
        - corrected the subagent-derived ledger evidence for rows 33–35: the `composes_at` exclusion for `schema(mapping)` predates this cycle and is not claimed as part of this repair
        - behavior change recorded: a `#` line no longer ends the holder scan, since inside a quoted or block scalar it is content; a comment still ends a plain scalar because the shared decoder enforces that; the change is documented in `darkmatter/docs/topics/schemas/dmls-schema-support.md`

### Completion Report

- **outcome: complete**; elapsed about 15 min (20:17:09 → 20:32:28 local/UTC)
- parent findings: total 1 = fixed 1 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
        - instance rows: 41 review rows (fixed: 2, 4–8, 10, 12, 14, 16, 18, 20, 22, 32–36; clean: 1, 3, 9, 11, 13, 15, 17, 19, 21, 23–31, 37–41) plus sweep rows S1–S2 (fixed)
- verified repairs:
        - R3-F1: before reading a `key:` or `- ` line, DMLS first asks the shared `simplified_schema::decode_scalar_node_prefix` whether an earlier scalar reaches the cursor. This applies to a property on the cursor's own line and to one on a line passed during the backward scan. The outermost holder that reaches the cursor owns the line. Each consumer uses that owner: the composition cursor, eligibility filtering, the receiving-object scope, and tolerant namespace recovery. So spreads after an explicit property complete only the eligible `base` (or `this`) in quoted (closed or open), folded, and literal scalars, in both hosts and in every owner position.
        - S1: ordinary data-value completion no longer reads text inside a data scalar as a key
        - S2: a `#` line inside a quoted or block definition no longer ends holder discovery
- open findings: none
- no `human-review-items` added; no decision met the author-time threshold
- changed files this cycle (all R3-F1):
        - `darkmatter/dmls/src/providers/frontmatter.rs`
        - `darkmatter/dmls/src/providers/frontmatter/composition_tests.rs`
        - `darkmatter/dmls/tests/l1/object_composition_editor.rs`
        - docs: `darkmatter/docs/topics/schemas/dmls-schema-support.md`, `.claude/skills/darkmatter/schema.md`
        - the simplified-schema library was not changed this cycle
- final verification (run by the orchestrator after all changes; no subagent running; load average 12.5 on 16 cores at the end of the test run):
        - darkmatter `just test` (library, CLI, DMLS, Zed): 17552/17552, 10 skipped, "none at or above 5.0s" (R3-F1 and every earlier finding's editor tests)
        - darkmatter `just lint`: clean, including the zed-dmls wasm32-wasip2 check
        - simplified-schema `just test`: 2485/2485 (work-counters/terminal) and 2105/2105 (`--no-default-features`), none at or above 5.0s; `just lint` clean (regression check on the feature area)
        - `just check-tier-coverage darkmatter` and `just check-tier-coverage simplified-schema`: stranded (0)
        - `just slow-tests fresh`: fresh
        - subagent-scoped runs: DMLS filter 371/371, DMLS `--lib --test l1` 2104/2104; `cargo clippy -p darkmatter-lsp --all-targets -D warnings` clean
        - cross-OS check not run: the change handles strings and cursors, has no path or OS-specific code, and CRLF is covered by `composition_operands_after_an_explicit_property_on_crlf_continuation_lines`
- no commit, no formatter, no terminal or browser window was opened
