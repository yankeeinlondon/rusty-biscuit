---
spec: "/home/build/wt/rusty-biscuit/fix-biscuit-terminal/simplified-schema/features/2026-11-22-object-composition/spec.md"
plan: "simplified-schema/features/2026-11-22-object-composition/plan.md"
implemented-by: "claude/opus"
started-phase: "1"
source_files_during_phase_1:
    - simplified-schema/lib/src/lib.rs
    - simplified-schema/lib/src/resolve.rs
    - simplified-schema/lib/src/simplified/grammar.rs
    - simplified-schema/lib/src/simplified/mod.rs
    - simplified-schema/lib/src/simplified/source.rs
    - simplified-schema/lib/src/simplified/types.rs
    - simplified-schema/lib/src/yaml_documents/semantic.rs
    - simplified-schema/lib/tests/l1/main.rs
    - simplified-schema/lib/tests/l1/object_composition.rs
    - simplified-schema/lib/tests/l1/shipped_corpus.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/named_types.rs
    - darkmatter/dmls/tests/l1/claudine_schema_discovery.rs
    - darkmatter/lib/tests/l1/schemas_source_projection.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - simplified-schema
    - darkmatter
    - darkmatter-lsp
decisions-for-review:
    - |-
        A `types` table reads `$extends` as an ordinary type name only in the
        standalone parsers, as the plan says (table mode for the `types`
        carrier in `parse_standalone_schema_payload_with_source` and in
        `locate_standalone_definition_failure`). The plan did not foresee that
        envelope admission checks `types` first with the `schema(mapping)`
        meta-type, which parses the table as a schema object. So a `types`
        entry named `$extends` passes admission only when its definition is
        also a valid composition operand (for example `$extends: other`);
        `$extends: string` is rejected at admission with the reserved-directive
        message. I left the envelope contract alone because the plan says not
        to touch it. Alternatives: add a table-reading option to the `schema`
        meta-type and use it for `types` in the shipped envelope definitions
        (an envelope-contract change), or drop table mode and reserve
        `$extends` in `types` tables too (simpler, but a table key that is
        then silently treated as composition of the table). Redoing either
        touches `MappingMode` in `simplified/mod.rs`, its two callers in
        `simplified/source.rs`, and the test
        `composition_extends_in_a_types_table_is_an_ordinary_type_name`.
    - |-
        `SchemaShape::composition` is `Option<Box<ObjectComposition>>`, not the
        `Option<ObjectComposition>` the plan names. Unboxed, it grew every
        `PropertyAtom` past clippy's `large_enum_variant` limit (lint failed on
        `PropertyDef`, the grammar's `Arm`, and the trigger `MatchExpr`). The
        box is invisible to Serde, so records are unchanged. Alternative: keep
        it unboxed and box the large enum variants instead, which changes
        widely used public enums. Readers match with
        `shape.composition.as_deref()`.
message-to-agent: |-
    Phase 1 landed the authored model only. `SchemaShape::composition` is
    `Option<Box<ObjectComposition>>` (match it with `.as_deref()`); a spread's
    `literals_before`/`patterns_before` count the members written before it.
    Resolution does NOT expand composition yet: `ImportEngine::expand_shape`
    in `resolve.rs` sets `composition: None`, so today a composed object
    silently loses its operands when resolved or lowered. Phase 2 must
    replace that with real assembly and add the lowering guard (Rule 12).

    Parsing entry points: `grammar::parse_extends_operand` (whole `$extends`
    text) and `Parser::parse_composition_operand` (shared with `...`).
    `MappingMode::{Object, Table}` in `simplified/mod.rs` decides whether
    `$extends` is reserved; only the standalone `types` carrier uses `Table`.
    Source paths: `path.spread(i)` for inline spreads; `$extends` operands
    are at `path.property("$extends")` (scalar) or `.union_arm(i)` (list
    items), with `SchemaSpanKind::CompositionOperand`, `ImportName`, and
    `ImportReference` spans. Phase 1 tests are in
    `simplified-schema/lib/tests/l1/object_composition.rs` (all named
    `composition_*`); extend that file in Phase 2 as the plan says.

    Deferred from Phase 1's robustness matrix to Phase 2 because they need
    resolution: the positive Markdown control through
    `SimplifiedSchema::try_from_markdown` (the invalid Markdown case is in
    place) and the trigger-payload field-wiring case through
    `resolve_embedded_trigger_payload`/`prepare_trigger_bundle`.

    The tree at HEAD has pre-existing failures that are not from this
    feature. Commit 8f33de0d7 deleted
    `claudine/schemas/sequence-types.yaml` and moved its types into the
    `sequence.yaml` trigger's `types:` table, which trigger envelopes do not
    support yet (host-registration-v2). Phase 1 removed the dead
    `include_str!` from two tests so they compile
    (`simplified-schema/lib/tests/l1/shipped_corpus.rs` and
    `darkmatter/dmls/tests/l1/claudine_schema_discovery.rs`), but these
    still fail with or without this feature's changes:
    `shipped_corpus::claudine_bundle_and_its_triggers_resolve_from_supplied_sources`
    (simplified-schema), three `shipped_schema_corpus::*` tests (darkmatter,
    about `lifecycle-events.yaml` and `sequence.yaml` trigger keys), and
    `claudine_schema_discovery::removed_sequence_keys_are_reported_at_their_paths`
    (darkmatter-lsp). Claudine itself does not compile at HEAD
    (`claudine/lib/src/composition/schema/bundle.rs` and three Claudine
    tests still `include_str!` the deleted file), so Phase 5's
    `cargo check -p claudine …` fails until that is fixed.
---

# Implementation Log for 2026-11-22-object-composition (5 phases)

## Phase 1

### What was built

- **Authored model** (`simplified/types.rs`): `SchemaShape::composition`
  (`Option<Box<ObjectComposition>>`, Serde default, omitted when `None`),
  `ObjectComposition::{Inline(Vec<InlineSpread>), Extends(Extends)}`,
  `Extends::{Scalar, Sequence}`, `InlineSpread { literals_before,
  patterns_before, operand }`, and `CompositionOperand { name, reference:
  Option<String> }` (`None` keeps a bare name; `resolved_reference()` returns
  `this` for it). Reexported from `simplified` and the crate root, and named
  in the AST module docs. The resolver's rebuilt shape sets the field to
  `None`. Darkmatter's `SchemaShape` literals already used
  `..Default::default()`, so they needed no change.
- **Grammar** (`simplified/grammar.rs`): one operand reader,
  `Parser::parse_composition_operand`, is used by `...` in
  `parse_inline_object_body` and by the new
  `parse_extends_operand`, which parses a whole `$extends` string. It reads
  the name with the lexer's identifier rule and the path with `read_fileref`
  and its delimiters for the current nesting. It rejects these with located
  grammar errors: `(`/`[` postfixes, `->` descriptions, `{{ … }}` slots, empty
  names, and empty paths. A bare built-in name gets a hint to write
  `Name@this`. The `$extends` form also says the directive is reserved, that
  it needs a named object, and that raw JSON Schema can declare a literal
  key. Trailing text after a `$extends` operand is an error, with a
  sequence hint when it starts with `,`. The duplicate-literal check runs
  across spreads unchanged.
- **YAML directive** (`simplified/mod.rs`): `parse_schema_shape` takes a
  `MappingMode`. In `Object` mode, `$extends` (quoted or not) is parsed by
  `parse_extends`: one string, or a nonempty sequence of strings. A null,
  number, boolean, mapping, or empty sequence is an error at the
  `$extends` path; a non-string item is an error at `.union_arm(i)`. Nested
  mappings, mapping-valued named declarations, and root-union and
  property-union mapping arms all go through `parse_mapping_atom` or the
  root-union arm path, so they share the directive. `Table` mode is used
  only for the standalone `types` carrier
  (`parse_standalone_schema_payload_with_source` covers `parse_standalone`
  and `rejected_envelope`; `locate_standalone_definition_failure` is the
  source-failure fallback).
- **Source addressing** (`simplified/source.rs`): `SchemaSourcePathSegment::Spread(usize)`,
  `SchemaSourcePath::spread`, a `[...N]` display form, and
  `SchemaSpanKind::CompositionOperand`. `project_shape_source` projects a
  `$extends` value (its Definition span, and each item's UnionArm and
  Definition spans). `project_inline_shape_range` projects each `...`
  operand at `path.spread(i)`. Both record CompositionOperand, ImportName,
  and (for an explicit `@`) ImportReference through `DecodedScalar`, so YAML
  quoting, folding, and escapes are crossed. An alias operand records only
  its own span. The located-suggestion walker skips the directive key.
  Exhaustive segment matches in `yaml_documents/semantic.rs` and two DMLS
  diagnostics modules stop at a spread, which is text inside a scalar, not a
  YAML node.

### Requirement-to-test mapping

All cases are in `simplified-schema/lib/tests/l1/object_composition.rs`
(the L1 binary, minimal-feature build except the Markdown case, which needs
`feature = "file"`). They call public APIs in process: `parse_property_definition`,
`parse_yaml_schema`, `parse_schema_declaration_with_source`,
`parse_standalone_schema_document`, `locate_standalone_definition_failure`,
`SimplifiedSchema::try_from_markdown`, and `triggers::parse_trigger_envelope_from_str`.

| Requirement | Tests |
|---|---|
| Spread positions and counts; spreads-only; repeats; trailing comma | `composition_spread_records_its_position_among_literal_and_pattern_members`, `composition_spread_may_be_the_whole_body_repeat_and_end_with_a_comma` |
| Whitespace normalization | `composition_spread_whitespace_does_not_change_the_shape` |
| Absent composition omitted from the shape and its record; composed record round-trips | `composition_absent_is_omitted_from_the_shape_and_its_record` |
| Bare built-in rejected; `Name@this` selects the local name | `composition_keyword_collision_needs_an_explicit_local_reference`, `composition_extends_rejects_an_invalid_operand_at_the_operand` |
| Postfixes, slots, empty name or path rejected at their span | `composition_spread_rejects_every_postfix_and_an_empty_operand` |
| Receiving postfix and description stay on the containing atom | `composition_receiving_postfix_stays_on_the_containing_atom` |
| Duplicate literal error across a spread | `composition_keeps_the_duplicate_literal_error_across_a_spread` |
| Path punctuation stays path text; a group's `)` ends it | `composition_import_path_punctuation_follows_the_reference_delimiters` |
| Spreads in arrays, unions, and nested objects | `composition_spreads_parse_in_arrays_unions_and_nested_objects` |
| `$extends` scalar, flow list, block list (equal) | `composition_extends_takes_a_scalar_a_flow_list_or_a_block_list` |
| Placement independence; quoted key keeps its reserved meaning | `composition_extends_placement_and_key_quoting_do_not_matter` |
| Every schema-object position (root, nested, named declaration, property-union arm, root-union arm) | `composition_extends_is_reserved_in_every_schema_object_position` |
| `types` table keeps `$extends` as a type name | `composition_extends_in_a_types_table_is_an_ordinary_type_name` |
| Field matrix: control | `composition_extends_field_control_parses` |
| Field matrix: explicit null (`null`, `~`), number, boolean, mapping, empty sequence, empty string at the value | `composition_extends_rejects_a_wrong_carrier_at_the_value` |
| Field matrix: implicit null (empty range just past the key, distinct from explicit null) | `composition_extends_implicit_null_is_located_where_its_value_would_be` |
| Field matrix: mixed list, all-invalid list, null and sequence items, at the item | `composition_extends_rejects_a_wrong_item_at_the_item` |
| Field matrix: invalid scalar and list operands from the shared reader | `composition_extends_rejects_an_invalid_operand_at_the_operand` |
| Carrier routes, one invalid case each (nested, named declaration, property-union arm, root-union arm) | `composition_extends_rejects_invalid_fields_in_every_carrier_route` |
| Markdown frontmatter wiring keeps the grammar error | `composition_extends_in_markdown_frontmatter_keeps_the_grammar_error` |
| Standalone admitted YAML loader: duplicate `$extends` (plain and quoted), trailing garbage, second document | `composition_standalone_loader_rejects_duplicate_trailing_and_multiple_documents` |
| Trigger envelope loader: positive control plus the same three document shapes | `composition_trigger_envelope_loader_rejects_duplicate_trailing_and_multiple_documents` |
| Inline operand spans; repeated operands are distinct | `composition_spread_spans_distinguish_repeated_operands` |
| Spans cross folding and escapes; a rejected operand is located through quoting | `composition_spread_spans_cross_yaml_folding_and_escapes` |
| `$extends` spans: quoted key, quoted scalar, flow and block items, repeats | `composition_extends_spans_cover_scalars_and_each_list_item` |
| Root-union arm scopes | `composition_extends_spans_in_root_union_arms_follow_the_arm` |
| Malformed operands located in the authored text | `composition_malformed_operands_are_located_in_the_authored_text` |
| `Spread` path is distinct and renders `[...N]` | `composition_path_renders_spreads_distinctly` |

Code-smell audit of the new parse paths: no `#[serde(default)]` on a
load-bearing input field (the only one is the AST field's default, which
keeps prepared records compatible and sits after input validation), no
`Option` that merges absent and null (`$extends:` and `$extends: null` both
reach `parse_extends` as `YamlValue::Null` and are rejected; absence is the
key being missing), and no `filter_map`, `unwrap_or_default()`, or `.ok()`
on a parse result.

Two loader notes. The YAML loader reports no position for a second
document, so the test asserts rejection without a location for that row.
For a duplicate key or trailing garbage, the standalone wrapper keeps the
loader's line and column in its message, not in `span`; the test accepts
either form.

### Verification

- `cargo nextest run -p simplified-schema -E "test(composition)"`: 29/29 pass.
- `cargo check -p simplified-schema --no-default-features --lib`: compiles,
  and the minimal L1 run below includes the new cases.
- `cargo check -p darkmatter -p darkmatter-lsp -p simplified-schema --all-targets`: clean.
- `just test --no-fail-fast` (simplified-schema; `just test` stops at the
  first failure and skips the minimal leg, so I ran both legs to completion):
  2416/2417 pass, none at or above 5.0 s (load average about 9–12). The one
  failure is the pre-existing
  `shipped_corpus::claudine_bundle_and_its_triggers_resolve_from_supplied_sources`.
  To prove it is pre-existing, I reversed only this phase's `src/` patch,
  ran it alone, saw the same failure (`unknown named type steps in
  embedded claudine/sequence.yaml`), and reapplied the patch.
- `just test-minimal --no-fail-fast`: 2047/2048, the same single
  pre-existing failure.
- `just lint`: passes. The first run failed on `large_enum_variant`, which
  boxing `composition` fixed.
- `just check-tier-coverage simplified-schema`: nothing stranded.
- Dependents, run serially and limited to the slices that use the changed
  symbols:
  - `cargo nextest run -p darkmatter -E 'test(/schema/)'`: 1962/1965. The 3
    failures are pre-existing `shipped_schema_corpus::*` tests about
    committed Claudine schemas that use trigger keys (`description`,
    `types`) the envelope does not admit yet.
  - `cargo nextest run -p darkmatter-lsp -E 'test(/named_type|frontmatter|claudine_schema_discovery|source_path|diagnostic/)'`:
    419/420. The failure is the pre-existing
    `claudine_schema_discovery::removed_sequence_keys_are_reported_at_their_paths`;
    it expects the sequence `group` type that moved into the unsupported
    trigger `types:` table.
- No cross-OS check: the phase adds pure in-memory parsing and touches no
  paths, processes, or OS APIs.

### Pre-existing breakage found and partly fixed

Commit 8f33de0d7 deleted `claudine/schemas/sequence-types.yaml`, but
several `include_str!`s still named it, so the simplified-schema and
darkmatter-lsp L1 binaries did not compile at HEAD. I removed the dead
entries from `simplified-schema/lib/tests/l1/shipped_corpus.rs` and
`darkmatter/dmls/tests/l1/claudine_schema_discovery.rs` (the `sequence.yaml`
entry carries those types now). I left Claudine's own references alone
(`claudine/lib/src/composition/schema/bundle.rs` and three Claudine tests):
they are outside this feature, and the fix belongs with the trigger `types`
support in host-registration-v2.

### Process note

While this phase was running, the author's tooling committed the source
changes (`3b66ddc90 feat(simplified-schema): add object composition
syntax`) and the plan and log (`9f6ef9e9d`). This agent made no commits.
The committed tree includes the final boxed field. The log body and the
plan/log frontmatter written after that are uncommitted.
