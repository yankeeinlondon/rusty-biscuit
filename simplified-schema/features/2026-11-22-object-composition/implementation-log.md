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
    - |-
        Phase 2: a copied member's origin reaches the lowerer through a hidden
        field, `SchemaShape::composed_sites` (`#[doc(hidden)]`,
        `#[serde(skip)]`, and equal to every other value under `PartialEq`),
        not through a transient map passed beside the shape as the plan
        describes. Lowering takes a bare `SchemaShape` from many callers, so a
        side map would have meant threading a new parameter through every
        lowering entry point. The field is never serialized and never
        affects equality, so records and comparisons are unchanged.
        Alternative: a side table keyed by shape identity, threaded through
        `object_body_from_shape` and its callers.
    - |-
        Phase 2: the content-only no-file check extends `external_reference`
        (in `schema/layer.rs`) to composition operands rather than adding a
        no-file namespace mode. It runs before any probe or read, as the
        plan requires, but it is conservative: an operand that names another
        file makes the schema unpreparable from content even when every member
        it contributes is replaced. That matches Rule 7, which resolves an
        operand even when it contributes nothing.
    - |-
        Phase 2: assembled results are cached per `(DefKey, depth)`, so the
        depth limit stays exact when the same target is reached at different
        depths. `InvalidCompositionTarget.name` is the operand as written
        (for example `alias`), and `definition` points at the declaration
        that decided the kind. Target kinds: a tuple is reported as a list
        (`Array`), and only `object` and `$ref` are `OpaqueObject`; every
        other primitive is `Primitive`.
    - |-
        Phase 3: in a property union written as a YAML sequence, only an arm
        whose object uses `$extends` is exported as a mapping; every other
        arm stays a type-expression string, as before. The plan says to
        "route mapping unions through write_atom". The parsed model does not
        record whether an arm was authored as a mapping, and writing every
        object arm as a mapping would change the export of schemas that
        do not compose. Alternative: export every inline-object arm through
        the block writer, which changes existing output. Redoing it is a
        one-line change in `write_def` (`schema/export.rs`) plus the
        expectations in `object_composition_export.rs`.
    - |-
        Phase 3: when `select` takes a named type from a root that has no
        `types` table (the root's own properties act as the table) and that
        root uses `$extends`, the copied table drops the root's `$extends`.
        The plan does not cover this case. Otherwise the export would write
        `$extends` inside `types:`, where it reads back as an ordinary type
        name rather than a directive. The root's operands are not
        declarations, so the set of named types is unchanged. Alternative:
        refuse such an export with `ExportUnsupported`. The change is in
        `Layer::select` (`schema/layer.rs`).
    - |-
        Phase 3: the composition cursor role is
        `CompositionOperand { part: Name | Reference { name }, operand_span }`.
        A spread operand's cursor path ends in `Spread(i)`, the same address
        the source map uses for that operand. `token_span` covers the
        active half (the name, or the text after `@`), and `operand_span`
        covers the whole operand up to the cursor. The `$extends`
        entry points read one operand the caller has already framed, and
        report a root path. The plan asked only for "name versus
        import-path state and the raw replacement range". Alternatives:
        reuse the existing `ImportReference` role for the reference half
        (DMLS could then not tell a composition reference from an ordinary
        import), or leave out `operand_span`. Changing this touches
        `simplified/cursor.rs` and its tests, and the Phase 4 DMLS consumer.
    - |-
        Phase 3: changing `SchemaShape` in Phase 2 made Darkmatter's
        `Declaration::Found(Cow<PropertyDef>)` trip clippy's
        `large_enum_variant` lint. The `Declaration` enum is a short-lived
        lookup result, and I allowed the lint on it, as Darkmatter already
        does for several similar enums. Alternatives: box the `Found`
        payload (one allocation per lookup; 8 call sites in
        `projection.rs` and `typing.rs`), or shrink `SchemaShape` by merging
        `composition` and `composed_sites` into one boxed field.
    - |-
        Phase 4: a Markdown document's inline `$schema` is now resolved with
        the open editor buffers and with the resolver's read and probe
        records, so an unsaved edit of an imported file, or a missing import
        that appears, refreshes the importer. Before, that path read only
        disk and recorded no probes, which also affected ordinary
        `Name@file` imports. Every dependency the overlay cache records for
        such a document is now hashed from the open buffer when one exists.
        Alternative: keep Markdown importers on disk, which contradicts the
        spec's unsaved-refresh requirement. Undoing it is about 30 lines in
        `overlay/mod.rs` (`schema_for`, `collect_dep_hashes`).
    - |-
        Phase 4: `SchemaError::ComposedMember` (a lowering failure inside a
        copied member) is reported with the named-type code
        `dm.schema.invalid_type_definition` at the member's declaration,
        like the other composition errors, rather than as
        `dm.schema.prepare` on the whole `$schema`. Reverting is a one-line
        change in `diagnostics/named_types.rs::is_named_type_error`.
    - |-
        Phase 4: the editor's import catalog follows alias and composition
        targets of imported definitions into further files, at most 32 files,
        resolving each from the importing file's folder but with the
        consuming document's schema roots, since the imported file's own
        roots are not known there. Only exported definitions that are
        aliases or that compose at their top level are followed, not targets
        nested inside their properties. A target that is not followed is
        treated as unknown, so completion keeps it. Alternative: follow every
        target in each imported file, which reads more files per keystroke.
    - |-
        Phase 4: `simplified_schema::simplified::grammar::parse_extends_operand`
        is now public, so DMLS can read a `$extends` item from a buffer that
        does not parse as a whole. Alternative: a new public wrapper with a
        narrower signature. Either way it is one function.
    - |-
        Phase 4: `$extends` completion is routed by a check in
        `providers/frontmatter.rs` that runs before the meta-schema kind
        lookup, rather than by adding a new kind to `MetaSchemaKind` in
        `overlay/schema.rs`. While a buffer does not parse, local names come
        from the buffer with the cursor's line blanked and each entry parsed
        on its own; a one-line flow `$schema: {...}` then falls back to
        names without definitions, so nothing is filtered there. Moving the
        check into the kind lookup is a small change.
message-to-agent: |-
    Phase 4 landed DMLS operand completion, dependency tracking, diagnostics,
    the editor-session tests (`darkmatter/dmls/tests/l1/object_composition_editor.rs`),
    and the documentation. For Phase 5:

    - `darkmatter/docs/use-skills/use-dmls/schemas.md` has a new subsection
      "Reusing an object's properties"; `md compose` on it succeeds. Publish
      with `just publish-skills` and check the installed copy.
    - Pre-existing failures, unchanged by this phase:
      `simplified-schema` `shipped_corpus::claudine_bundle_and_its_triggers_resolve_from_supplied_sources`
      (reads the deleted `claudine/sequence.yaml` sequence types), plus the
      Darkmatter failures recorded in the Phase 4 log (`shipped_schema_corpus::*`,
      `darkmatter-cli` L1 not compiling because it `include_str!`s the deleted
      `claudine/schemas/sequence-types.yaml`, and
      `claudine_schema_discovery::removed_sequence_keys_are_reported_at_their_paths`).
      `claudine` itself does not compile (`composition/schema/bundle.rs`
      `include_str!`s the same deleted file), so Phase 5's root
      `cargo check -p claudine ...` will fail for that reason alone.
    - The documentation subagent observed that `md schema validate` rejects a
      `types` table entry named `$extends` even with a valid operand value
      (admission parses `types` as a schema object; see the Phase 1 decision).
      The docs say to avoid such a name.
    - Two passive DMLS paths still read disk: the re-assembly after a slot
      failure in `overlay/mod.rs::schema_for`, and the related-location text
      lookup in `diagnostics/named_types.rs::file`.
source_files_during_phase_2:
    - simplified-schema/lib/src/errors.rs
    - simplified-schema/lib/src/lib.rs
    - simplified-schema/lib/src/resolve.rs
    - simplified-schema/lib/src/resolve/composition.rs
    - simplified-schema/lib/src/resolve/graph.rs
    - simplified-schema/lib/src/schema/layer.rs
    - simplified-schema/lib/src/simplified/convert.rs
    - simplified-schema/lib/src/simplified/mod.rs
    - simplified-schema/lib/src/simplified/types.rs
    - simplified-schema/lib/src/triggers/assemble.rs
    - simplified-schema/lib/src/triggers/finding.rs
    - simplified-schema/lib/tests/l1/object_composition.rs
    - darkmatter/lib/tests/l1/schemas_named_type_merge.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - simplified-schema/lib/src/schema/export.rs
    - simplified-schema/lib/src/schema/layer.rs
    - simplified-schema/lib/src/simplified/serialize.rs
    - simplified-schema/lib/src/simplified/cursor.rs
    - simplified-schema/lib/src/simplified/mod.rs
    - simplified-schema/lib/src/lib.rs
    - simplified-schema/lib/tests/l1/main.rs
    - simplified-schema/lib/tests/l1/object_composition_export.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/projection.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/providers/frontmatter/named_references.rs
    - darkmatter/dmls/src/providers/frontmatter/composition_tests.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/named_types.rs
    - darkmatter/dmls/src/diagnostics/named_types.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/codes.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/object_composition_editor.rs
    - simplified-schema/lib/src/simplified/grammar.rs
docs_updated_during_phase_4:
    - simplified-schema/README.md
    - simplified-schema/docs/topics/language/objects-and-named-types.md
    - simplified-schema/docs/topics/language/index.md
    - simplified-schema/docs/topics/language/local-type-references.md
    - simplified-schema/docs/topics/diagnostics.md
    - darkmatter/docs/topics/schemas/parsing/grammar.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - darkmatter/dmls/docs/diagnostics.md
    - darkmatter/docs/use-skills/use-dmls/schemas.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/simplified-schema/SKILL.md
    - darkmatter/docs/use-skills/use-dmls/schemas.md
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

## Phase 2

### What was built

- **Assembly** (`lib/src/resolve/composition.rs`, new). `ImportEngine::assemble`
  turns a shape's `composition` into its winning members: inline spreads
  apply in place, between the literals and patterns written around them;
  `$extends` operands apply in order before the mapping's own members. A
  replaced literal keeps its first position (Rule 6). Patterns are
  deduplicated by their emitted key, computed against the final literal
  names with the same `emitted_pattern_key` the lowerer uses, so two
  spellings with the same regex (including one created by the
  literal-exclusion wrapper) collide and distinct overlapping patterns both
  stay. Each winner keeps its declaring namespace and source path
  (`Member`). Targets are classified (Rule 8): a union, list or tuple,
  primitive, opaque `object` or `$ref`, function, or slot is refused with
  `InvalidCompositionTarget`. Alias chains are followed for assembly only.
  Composition hops share the named-type depth limit (32), and a loop of
  spread and alias hops is a `NonDescendingCycle` that locates every edge.
  Assembled results are cached per `(DefKey, depth)`.
- **Resolver wiring** (`resolve.rs`, `resolve/graph.rs`). Graph discovery,
  both expansion passes, and the default walk read the assembled winners.
  Each member expands in its own namespace and path, and a member from
  another namespace is rehomed (`rehome_definition`) so its slots,
  examples, and defaults stay anchored in the declaring file (Rule 9).
  `shape_has_imports` counts composition, so the no-import fast path still
  applies to schemas with neither.
- **Origins and errors** (`errors.rs`, `triggers/finding.rs`). New
  `SchemaError::{InvalidCompositionTarget, CompositionReference,
  ComposedMember}` and the public `CompositionTargetKind`, with terminal
  `BlockError` arms. Lookup and resolution failures keep their cause under
  `CompositionReference`, and trigger classification walks into it.
- **Lowering** (`simplified/convert.rs`, `simplified/types.rs`). The shared
  object lowering refuses a shape whose `composition` is still present
  (Rule 12). A resolved shape carries a hidden, non-serialized
  `composed_sites` table, so a copied member's lowering failure is wrapped
  in `ComposedMember` naming its declaration.
- **Content-only reads** (`schema/layer.rs`). `external_reference` also
  returns a composition operand that names a file, so the content APIs
  refuse it with `ResolutionUnavailable` before any probe or read.

### Requirement → test mapping

All in `simplified-schema/lib/tests/l1/object_composition.rs` unless noted.
The `disk` module is `feature = "file"`.

| Requirement | Tests |
| --- | --- |
| Rule 5–6 placement and replacement | `composition_the_spec_example_behaves_like_its_handwritten_equivalent`, `composition_spread_applies_in_place_and_later_entries_replace_earlier_ones`, `composition_extends_applies_its_operands_in_order_before_the_mapping` |
| Whole-declaration replacement (requiredness, default, description, example) | `composition_replacing_a_member_replaces_its_whole_declaration`, `disk::composition_winning_examples_resolve_from_their_declaring_file` |
| Every YAML object position, root unions | `composition_extends_resolves_in_every_schema_object_position`, `composition_root_union_arms_keep_their_own_local_names` |
| Local, aliased, `@this` built-in collision, empty targets | `composition_targets_local_aliased_keyword_named_and_empty_objects` |
| Target qualifiers discarded; receiver constraints apply | `composition_copies_members_and_never_the_target_objects_own_qualifiers` |
| Pattern emitted-key collisions, overlap, catch-alls, literal precedence, pattern-only closure | `composition_patterns_replace_by_emitted_key_and_distinct_patterns_overlap`, `composition_catch_alls_replace_and_literals_keep_precedence` |
| Rule 8 ineligible targets | `composition_rejects_every_ineligible_target_at_its_operand` |
| Fully replaced operand still checked | `composition_operands_resolve_even_when_every_member_is_replaced` |
| Cycles, depth 32/33, mixed alias chain | `composition_a_loop_of_composition_and_alias_hops_reports_every_reference`, `composition_hops_count_toward_the_named_type_depth_limit` |
| Base/branch recursion, recursive target outside its cycle | `composition_a_cycle_through_a_child_value_is_ordinary_recursion`, `composition_a_recursive_target_outside_its_cycle_keeps_its_own_type`, `disk::composition_recursion_through_a_spread_works_across_two_files` |
| Authored namespace never gains copied names; `Layer::select` | `composition_copied_names_are_not_local_names` |
| Host-supplied locals | `composition_with_host_supplied_local_types` |
| Merged semantics: subtype, `DefinitionTable`, strict and soft validation | `composition_subtype_answers_like_the_handwritten_object`, `composition_the_recursive_comparison_uses_the_definition_table`, `composition_soft_validation_repairs_and_defaults_like_the_handwritten_object` |
| Rule 12 lowering guard; copied member failure at its declaration | `composition_lowering_refuses_an_unresolved_operand`, `composition_a_copied_members_lowering_failure_names_its_declaration` |
| Content-only APIs read no file; Markdown control | `composition_content_reads_no_file_for_an_imported_operand`, `composition_markdown_frontmatter_resolves` |
| Transitive imports and probes; failing target keeps cause | `disk::composition_imports_record_the_target_and_its_transitive_files`, `disk::composition_a_target_file_that_fails_keeps_its_cause` |
| Rule 9 origins: names, examples, defaults, slots | `disk::composition_a_copied_member_resolves_names_where_it_was_declared`, `disk::composition_winning_examples_resolve_from_their_declaring_file`, `disk::composition_default_files_name_the_winning_declaration`, `disk::composition_a_winning_slot_evaluates_in_its_declaring_file` (instrumented slot host) |
| Embedded payloads, runtime and prepared | `embedded::composition_resolves_in_an_embedded_trigger_payload_at_runtime_and_prepared`, `embedded::composition_failures_in_an_embedded_trigger_keep_their_cause`, unit test `triggers::assemble::…::composition_in_an_embedded_payload_reads_bundle_documents_only` (virtual reads vs empty disk dependencies) |
| Terminal `BlockError` arms | unit test `errors::…::composition_errors_name_the_operand_the_declaration_and_the_cause` (`terminal` feature, run by `just test`) |
| Darkmatter `AppliedDefault.origin` | `darkmatter/lib/tests/l1/schemas_named_type_merge.rs::composed_default_origin_names_the_file_of_the_winning_declaration` |

`composition_markdown_frontmatter_resolves` is gated on `feature = "file"`
because `try_from_markdown` and `SchemaSource` exist only there; the first
minimal run failed to compile until it was gated.

### Verification

- Checkpoint: `cargo nextest run -p simplified-schema -E 'test(/named_type|recurs|template|embedded|import|composition|shared_type|trigger/)'`:
  512/513, the one failure pre-existing (below). `test(composition_)`: 64/64.
- `just test --no-fail-fast` (default leg, `work-counters,terminal`):
  2452/2453, none at or above 5.0 s (load average about 10).
- `just test-minimal --no-fail-fast`: 2073/2074.
- `just lint`: passes.
- `just check-tier-coverage simplified-schema`: nothing stranded.
- Dependents, serially and filtered:
  - `cargo nextest run -p darkmatter -E 'test(/schema/)'`: 1963/1966.
  - `cargo nextest run -p darkmatter-lsp -E 'test(/named_type|schema|frontmatter|diagnostic/)'`: 679/680.
  - `cargo nextest run -p test-toolkit -E 'test(/schema/)'`: 5/5.
  - `cargo check -p claudine`: fails at HEAD (missing
    `claudine/schemas/sequence-types.yaml`), unchanged by this phase.
- `affected_scope.py` over the working tree: `darkmatter`,
  `simplified-schema`, `test-toolkit`, `claudine`.
- No cross-OS check: resolution and lowering are in-memory; the only file
  access goes through the existing namespace resolver, and the disk tests
  compare canonicalized paths.

### Pre-existing failures (unchanged from Phase 1)

`shipped_corpus::claudine_bundle_and_its_triggers_resolve_from_supplied_sources`
(simplified-schema, both legs), the three `shipped_schema_corpus::*` tests
(darkmatter; causes are `claudine/schemas/lifecycle-events.yaml` and
`sequence.yaml` trigger keys `description`/`types`), and
`claudine_schema_discovery::removed_sequence_keys_are_reported_at_their_paths`
(darkmatter-lsp). All are about committed Claudine schemas, not composition.

### Docs

No `docs/` page changed: the plan assigns the composition documentation and
skill updates to Phase 4.

## Phase 3

### What was built

- **Inline serializer** (`simplified/serialize.rs`). `write_inline_object`
  writes each `...Name` spread at its recorded position. For each spread
  it writes the literal and pattern members that come before it, then the
  spread, and it writes the remaining members at the end. Members between
  two spreads therefore come out literals first, the normalization Rule 1
  allows. An object with spreads also writes its pattern members, with the
  key text the block export uses. `pattern_key_text` moved here from
  `export.rs` so both writers share it. A new `composition_operand_text`
  writes `Name` or `Name@reference`. Spread-free objects serialize exactly
  as before.
- **YAML export** (`schema/export.rs`):
  - `write_shape` writes `$extends` first, in its scalar or sequence form.
  - `write_atom` keeps an object with spreads as a string, even when it
    would otherwise be written as a block mapping.
  - A property-union arm whose object uses `$extends` is written as a
    mapping, so the directive survives. Every other arm keeps the string
    form.
  - The loss check now lets an object with spreads carry pattern members.
    It still refuses `$constraints`, and pattern members on any object
    without spreads. It also refuses an object that uses `$extends` inside
    a type expression, which the parser cannot produce.
  - Layered export returns `ExportUnsupported` when any layer's root
    composes, or when a lower layer contributes a property that composes.
    A nested composition in the top layer is still exported.
  - Anchors, aliases, and folded scalars are written as their decoded
    operand text.
- **Derived-root selection export** (`schema/layer.rs`, `export.rs`).
  `AuthoredSchema` has a new `selected` field, which `Layer::select` sets
  to the selected name. When the selected root is an object with spreads,
  the export writes `$schema: { $extends: "Name@this" }` beside the
  authored `types` table. If `Name@this` does not read back through the
  operand grammar as that name (for example `type.v2`), the export
  returns `ExportUnsupported`. When the table is a root that uses
  `$extends`, `select` drops the root's directive from the copied table
  (see decisions-for-review).
- **Cursor** (`simplified/cursor.rs`; reexported from `simplified` and the
  crate root):
  - A new role, `SchemaCursorRole::CompositionOperand { part,
    operand_span }`, with `CompositionOperandPart::{Name, Reference {
    name }}`.
  - In an inline object, `...` at a key position starts an operand. The
    inline-object frame counts its spreads, so the cursor path ends in
    `Spread(i)`. A `,` returns the cursor to a key position.
  - New entry points `locate_composition_operand_cursor` and
    `locate_composition_operand_cursor_in_scalar` read one `$extends`
    scalar or framed sequence item. They use the same decoder
    (`decode_partial_scalar_at` / `DecodedScalar::prefix_at`) and the same
    lexer identifier rule as the existing locators, so name, `@this`, and
    file-path states share one authority for edit ranges.
- **DMLS** (`providers/frontmatter.rs`). The new variant gets a
  no-completion arm, so the exhaustive match still compiles and `...`
  offers nothing, as before. Phase 4 adds the real completion.
- **Darkmatter lint** (`markdown/compose/expression/scope/projection.rs`).
  Phase 2's `SchemaShape::composed_sites` pushed `Declaration` over
  clippy's `large_enum_variant` limit. The lint is now allowed on that
  enum, with a reason comment (see decisions-for-review).

### Requirement-to-test mapping

The export tests are in `simplified-schema/lib/tests/l1/object_composition_export.rs`,
registered in `tests/l1/main.rs`. Each case uses the public API in process
and round-trips: export, check that the reparsed authored model
(`parse_standalone_schema_document`: declaration plus `types`) equals the
original, check that the reparsed schema prepares equal JSON Schema, and
check that a second export is byte-identical.

| Requirement | Tests |
| --- | --- |
| Spread first, middle, last; spreads-only; repeats; trailing comma; replacement keeps first insertion position | `composition_export_writes_each_spread_at_its_position` |
| Pattern members around spreads; a pattern-only object with no spread is still refused where it must be a string | `composition_export_keeps_pattern_members_around_spreads` |
| Arrays, `\|` unions, flow-sequence arms, nested objects, pattern values, receiving postfix and description | `composition_export_keeps_spreads_in_arrays_unions_and_nested_objects` |
| Explicit `@this`, including a built-in collision (`string@this`) | `composition_export_keeps_explicit_local_references` |
| `$extends` scalar/sequence written before members; placement independence; property-union mapping arm; requiredness kept | `composition_export_writes_extends_before_the_mapping_members` |
| Root-union arm directives | `composition_export_keeps_root_union_arm_directives` |
| Anchor, alias, folded operand, aliased spread value: decoded on write | `composition_export_writes_decoded_operands_for_anchors_aliases_and_folds` |
| `Layer::select` validation (including the recursive child) and derived-root export; mapping-composed selection; source schema still exports its named declaration | `composition_export_of_a_selected_inline_composition_extends_the_selection` |
| Selection from a composed root drops the root's directive | `composition_export_of_a_selection_from_a_composed_root_omits_the_roots_directive` |
| Selected name not writable as an operand → `ExportUnsupported` | `composition_export_of_a_selection_whose_name_is_not_an_operand_is_unsupported` |
| Layered refusal (root composition in either layer, composing lower-layer property) and retained top-layer nested composition | `composition_export_refuses_layers_it_cannot_flatten` |
| Legacy output for objects that do not compose | `composition_export_leaves_uncomposed_objects_unchanged` |
| Imported operands with path punctuation reparsed through `try_from_yaml` with the original path and context; same imports recorded | `disk::composition_export_reparses_imported_operands_in_the_same_context` (`feature = "file"`) |
| Serializer unit round trip, canonical spellings | `simplified::serialize::tests::composition_spreads_round_trip_at_their_positions` |
| Cursor: empty and partial operand after `...`, before/after `@`, before a receiving postfix | `simplified::cursor::tests::composition_cursor_after_an_ellipsis_names_an_operand` |
| Cursor: nested objects, object arrays, flow and `\|` union arms | `composition_cursor_follows_nested_objects_arrays_and_union_arms` |
| Cursor: single/double-quoted, escapes, multiline plain scalars | `composition_cursor_projects_through_quoted_and_multiline_scalars` |
| Cursor negative controls: literal keys, ellipsis in a description, `...` outside an object or in a value, in an argument | `composition_cursor_ignores_keys_descriptions_and_document_data` |
| `$extends` operand entry points, quoted, offset, refusals, decoded multiline scalar with anchor | `composition_cursor_reads_a_whole_extends_operand` |

The Input Robustness Matrix does not apply: this phase adds no loader or
field validator. The export is a writer, and the cursor is a tolerant
editor reading.

### Verification

- Checkpoint: `cargo nextest run -p simplified-schema -E 'test(composition_export)'`
  13/13; `--lib -E 'test(composition_cursor)'` 5/5;
  `test(composition_spreads_round_trip)` 1/1.
- `just test --no-fail-fast` (simplified-schema, default leg): 2471/2472,
  `L1 run recorded: 2472 test(s), none at or above 5.0s` (load average
  about 5–8). The one failure is the pre-existing
  `shipped_corpus::claudine_bundle_and_its_triggers_resolve_from_supplied_sources`.
  `just test` stops after the default leg fails, so I ran the minimal leg
  separately.
- `just test-minimal --no-fail-fast`: 2091/2092, the same single
  pre-existing failure.
- `just lint` (simplified-schema): passes.
- `just check-tier-coverage simplified-schema`: nothing stranded.
- Dependents, serially and filtered:
  - `cargo nextest run -p darkmatter-lsp --no-fail-fast -E 'test(/named_type|schema|frontmatter|cursor|completion/)'`:
    658/659. The failure is the pre-existing
    `claudine_schema_discovery::removed_sequence_keys_are_reported_at_their_paths`.
  - `cargo nextest run -p darkmatter --no-fail-fast -E 'test(/schema|export|simplified_yaml/)'`:
    1974/1977. The failures are the pre-existing `shipped_schema_corpus::*`
    tests.
  - `cargo clippy -p darkmatter -p darkmatter-lsp --all-targets -- -D warnings`:
    clean after the `Declaration` allow. Before it, the library failed on
    `large_enum_variant`, a regression from Phase 2.
  - `just lint` in `darkmatter/` still fails at the `darkmatter-cli` leg.
    Its L1 tests do not compile because they `include_str!` the deleted
    `claudine/schemas/sequence-types.yaml`. This breakage was there at
    HEAD before this phase (see Phase 1).
- `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`:
  packages `darkmatter, darkmatter-lsp, simplified-schema, test-toolkit,
  claudine`, change_class `package`. Phase 2 is still uncommitted, so the
  working tree also holds its changes. The gates were therefore run.
- No cross-OS check. The export and cursor are in-memory string work, and
  the one disk test writes and reads a temporary directory through the
  existing resolver.

### Pre-existing limitation found

A block-mapping value written as a multi-line quoted scalar, such as
`p: '{ a: string,\n    n: number }'`, fails to load with "could not
project SimplifiedSchema expression spans through YAML source". This
happens with or without a spread, in the line locator in
`simplified/source.rs`. It is outside this feature, so I did not fix it.
The export test covers folded (`>-`), anchored, and aliased operands
instead, and all of them work.

### Docs

No `docs/` page changed: the plan assigns composition documentation to
Phase 4.

## Phase 4

### How the work was split

Three subagents ran in parallel, each limited to its own files. The
coordinator then wrote the session tests, registered them, fixed two stale
DMLS docs and the grammar module's EBNF, and ran the gates.

- **Operand completion** (`providers/frontmatter.rs`,
  `providers/frontmatter/named_references.rs`, new
  `providers/frontmatter/composition_tests.rs`).
  - The placeholder arm for `SchemaCursorRole::CompositionOperand` became
    real completion.
    - Before `@`: eligible local names (a keyword-colliding name is
      spelled `Name@this`) and imported `Name@target` names. The edit
      replaces the whole operand typed so far (`operand_span`).
    - After `@`: `this`, the files that export the name, and the same path
      suggestions `$schema:` values get. The edit replaces only the
      reference half.
    - No keywords, arrays, scaffolds or constraints are offered.
  - Filtering follows alias chains with a visited set, locally and into the
    catalog. It drops unions, arrays, primitives (including `object`),
    functions, slots, tuples, alias cycles, the receiving declaration, and
    objects whose local operands lead back to the receiver. A target it
    cannot see is kept. Completion runs no resolver and reads no file.
  - `$extends` values are read with the operand locators in the scalar,
    quoted-key, flow-sequence-item and block-sequence-item forms. This
    happens only in schema-object positions: standalone `$schema` and
    `types.X` mappings, the frontmatter `$schema` declaration, and
    frontmatter values whose recorded owner is a schema or type definition.
    A `$extends` data key and a `types` table key keep their old behavior.
  - The local scope keeps the exact implicit root-union arm.
  - Hover and go-to-definition already worked through
    `AuthoredDeclaration::reference_at`, because Phase 1 projects each
    operand's `ImportName` span. Tests now cover them.
- **Dependencies and diagnostics** (`overlay/{mod,named_types}.rs`,
  `diagnostics/{named_types,frontmatter,codes}.rs`; `parse_extends_operand`
  in `simplified/grammar.rs` made public).
  - The catalog collects inline-spread and `$extends` targets from the
    authored AST and from tolerant YAML, so one malformed sibling or item
    hides nothing else.
  - The catalog follows imported aliases and compositions into further
    files, at most 32 (`MAX_TRANSITIVE_FILES`), through the existing
    buffer-aware read. The new `NamedTypeCatalog::imported_from` gives
    access to those files.
  - Markdown `$schema` resolution (`OverlayCache::schema_for`) now uses
    open buffers and records the resolver's reads and probes, so an unsaved
    import edit or a newly created missing file refreshes the importer.
    This was a gap for every Markdown named-type import, not only for
    composition.
  - `file_failures` and the error-dependency walk look through
    `CompositionReference` and `ComposedMember` to the typed cause.
  - The three composition errors use `dm.schema.invalid_type_definition`.
    - `InvalidCompositionTarget` is ranged on the whole operand, with
      "`name` is defined here" related at the final declaration, even
      through an alias in another file.
    - An indirect failure ranges this document's reference into the other
      file and relates the operand there.
    - Operands are told apart by source path, never by spelling.
- **Documentation**: listed under `docs_updated_during_phase_4`. The main
  page (`objects-and-named-types.md`) gained an "Object Composition"
  section. It covers both forms, lookup, order and replacement, origin,
  constraints and patterns, the eligible targets and error table,
  recursion, export, migration, and DMLS behavior, with a Mermaid flow. The
  page's old claim that nested YAML mappings are rejected was corrected to
  match `parse_mapping_atom`. The coordinator also updated
  `darkmatter/docs/topics/schemas/dmls-schema-support.md` (code table and
  notes 7 and 8) and `darkmatter/dmls/docs/diagnostics.md`
  (`relatedInformation`), which the diagnostics subagent reported as stale.
  The grammar module's EBNF (`simplified/grammar.rs` `//!`) now has the
  `spread` member. Publication of the caller skill is left to Phase 5.

### Requirement-to-test mapping

The session tests are in `darkmatter/dmls/tests/l1/object_composition_editor.rs`,
registered in `tests/l1/main.rs` (the L1 binary). They run in-process over
`LspFixture` with no editor or terminal.

| Requirement | Test |
| --- | --- |
| Completion after `...` (empty and partial), in a scalar `$extends`, a quoted key and value, a flow item and a block item. Labels are exact, and ineligible primitive, union, array, opaque `object` and alias-cycle names, plus the receiver, are filtered. `string@this` is offered for a keyword collision. The edit range covers the typed operand | `operand_completion_offers_eligible_objects_in_both_forms` |
| An imported `Name@./file` offered for a partial flow item, ineligible imported targets filtered, and a reference-half edit that starts after `@` | `operand_completion_offers_imported_names_and_the_reference_half` |
| Implicit root-union arm scopes (frontmatter), and a standalone `types` table filtered for `$extends` | `operand_completion_keeps_implicit_root_union_arms_apart` |
| Hover and go-to-definition on a spread, a flow item, and an imported operand | `hover_and_definition_follow_operands_of_both_forms` |
| Of two operands into one file, only the ineligible second one is ranged, with the target's declaration as related information | `an_ineligible_second_operand_into_a_shared_file_is_reported_on_that_operand` |
| Importer and imported file both open with their disk text. A buffer edit to a list reports the error; editing back to an object with a renamed and a newly required member clears it, judges `id: a` in the consuming document as a type mismatch at its value, and offers `code` with `price` absent | `an_unsaved_imported_definition_edit_refreshes_the_importer` |
| A missing imported file is reported on the operand and clears once the file is created and reported through the watched-file path | `a_missing_imported_file_appearing_clears_the_operand_error` |

The subagents' unit tests:

- `providers::frontmatter::composition_tests::*` (18 tests). They cover
  each cursor form, filtering, the receiver rule, root-union arm scope,
  negative controls (a `$extends` data key, a `types` table key, no
  keywords or scaffolds), hover and definition, and flow-item framing.
- `overlay::named_types::tests::{composition_operands_are_targets_of_the_authored_declaration, tolerant_collection_keeps_operands_beside_malformed_members, imported_aliases_and_compositions_hydrate_their_files}`.
- `overlay::tests::{a_missing_composition_target_keeps_its_cause_and_its_appearance_refreshes, an_unsaved_edit_of_a_composed_file_refreshes_the_importer, a_composition_target_file_failure_is_located_in_that_file}`.
- `diagnostics::frontmatter::tests::{an_ineligible_local_composition_target_ranges_the_whole_operand, an_ineligible_imported_target_ranges_its_own_operand_and_relates_the_terminal_declaration, a_missing_composition_file_ranges_the_operand_with_the_named_type_code, an_indirect_composition_failure_navigates_back_to_the_receiver}`.

The Input Robustness Matrix does not apply. This phase adds no loader or
field validator: completion and the tolerant catalog are passive editor
readings, and `$extends` validation is the Phase 1 validator, which is
unchanged.

Not covered: `ComposedMember` routing has no direct test, because no simple
fixture triggers that error.

### Verification

- Subagent slices:
  - `cargo nextest run -p darkmatter-lsp --lib -E 'test(/composition|frontmatter|named_ref|completion/)'`:
    277 passed.
  - `... -E 'test(/named_type|composition|overlay|diagnostic/)'`: 314 passed.
  - `cargo nextest run -p simplified-schema -E 'test(composition)'`: 83 passed.
- Checkpoint:
  `cargo nextest run -p darkmatter-lsp --test l1 -E 'test(object_composition_editor)'`:
  7/7, each under 0.2 s.
- `simplified-schema`:
  - `just test --no-fail-fast`: 2471/2472, `L1 run recorded: 2472
    test(s), none at or above 5.0s`.
  - `just test-minimal --no-fail-fast`: 2091/2092.
  - The one failure in both legs is the pre-existing
    `shipped_corpus::claudine_bundle_and_its_triggers_resolve_from_supplied_sources`
    ("unknown named type `steps` in embedded `claudine/sequence.yaml`").
  - `just lint`: passes.
  - `just check-tier-coverage simplified-schema`: nothing stranded.
- `darkmatter`:
  - `just test --no-fail-fast` does not get past compilation, because
    `darkmatter-cli`'s L1 tests `include_str!` the deleted
    `claudine/schemas/sequence-types.yaml` (removed in `8f33de0d7`, before
    this feature). So I ran the recipe's other three packages with its
    features:
    `cargo nextest run --no-fail-fast -p darkmatter -p darkmatter-lsp -p zed-dmls-cli --features darkmatter/effects-instrumentation,darkmatter/work-counters,darkmatter-lsp/effects-instrumentation`.
    Result: 14875/14879, 10 skipped, no `SLOW` lines, load average about 2–3.
  - The four failures are the pre-existing ones from Phase 3: three
    `darkmatter::l1 shipped_schema_corpus::*` tests and
    `darkmatter-lsp::l1 claudine_schema_discovery::removed_sequence_keys_are_reported_at_their_paths`.
  - `just lint`: the library leg passes, and the CLI leg fails on the same
    `include_str!`.
    `cargo clippy -p darkmatter-lsp -p zed-dmls-cli --all-targets -- -D warnings`:
    clean.
  - `just check-tier-coverage darkmatter` cannot list the area for the same
    reason (`darkmatter-cli` does not compile). No new test or module name
    carries a tier marker, and the session tests are declared in the
    `tests/l1/main.rs` binary.
- Dependents: `cargo check --locked -p claudine -p claudine-cli` fails on
  `claudine/lib/src/composition/schema/bundle.rs`, which `include_str!`s
  the same deleted file. That predates this feature, and `claudine/` is
  untouched. The only library change this phase is making
  `parse_extends_operand` public, which cannot break a consumer.
- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  gives packages `darkmatter-lsp, simplified-schema, test-toolkit,
  claudine, darkmatter, darkmatter-cli` and change_class `package`, so the
  gates were run. Earlier phases are still uncommitted.
- No cross-OS check. The new code is in-memory editor logic. The tests'
  file use is temporary files through the existing resolver, and the
  expected paths come from `biscuit_file::canonicalize_simplified`.
