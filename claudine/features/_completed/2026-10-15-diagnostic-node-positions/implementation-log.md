---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/claudine/features/2026-10-15-diagnostic-node-positions/spec.md"
plan: "claudine/features/2026-10-15-diagnostic-node-positions/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/schemas/positions.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/mapping_orders.rs
    - darkmatter/lib/src/markdown/schemas/tests/mod.rs
    - darkmatter/lib/src/markdown/schemas/tests/positions.rs
    - darkmatter/lib/tests/l1/meta_schema_phase4.rs
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/enum_member_invalid/expected.json
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/missing_required/expected.json
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/missing_required/expected.pretty
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/property_union_invalid/expected.json
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/property_union_invalid/expected.pretty
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/property_union_nested_object/expected.json
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/property_union_nested_object/expected.pretty
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/root_union_none_match/expected.json
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/root_union_none_match/expected.pretty
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/Cargo.toml
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/diagnostic_positions.rs
    - darkmatter/cli/tests/l1/schema_validate.rs
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/enum_member_invalid/expected.pretty
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/missing_required/expected.pretty
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/property_union_invalid/expected.pretty
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/property_union_nested_object/expected.pretty
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/root_union_none_match/expected.pretty
    - darkmatter/lib/tests/position_fixtures/mod.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/diagnostic_positions.rs
    - darkmatter/lib/tests/l1/content_policy_editor_schema.rs
    - darkmatter/lib/src/markdown/schemas/tests/positions.rs
    - darkmatter/dmls/Cargo.toml
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/frontmatter/position_parity_tests.rs
docs_updated_during_phase_3:
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/inline/schema-validation.md
    - darkmatter/docs/errors/file-reference-failures.md
    - darkmatter/README.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/schema.md
source_files_during_phase_4: []
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
packages:
    - darkmatter
    - darkmatter-cli
    - dmls
source_code:
    - darkmatter/lib/src/markdown/schemas/positions.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/mapping_orders.rs
    - darkmatter/lib/src/markdown/schemas/tests/mod.rs
    - darkmatter/lib/src/markdown/schemas/tests/positions.rs
    - darkmatter/lib/tests/l1/meta_schema_phase4.rs
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/enum_member_invalid/expected.json
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/missing_required/expected.json
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/missing_required/expected.pretty
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/property_union_invalid/expected.json
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/property_union_invalid/expected.pretty
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/property_union_nested_object/expected.json
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/property_union_nested_object/expected.pretty
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/root_union_none_match/expected.json
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/root_union_none_match/expected.pretty
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/Cargo.toml
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/diagnostic_positions.rs
    - darkmatter/cli/tests/l1/schema_validate.rs
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/enum_member_invalid/expected.pretty
    - darkmatter/lib/tests/position_fixtures/mod.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/diagnostic_positions.rs
    - darkmatter/lib/tests/l1/content_policy_editor_schema.rs
    - darkmatter/dmls/Cargo.toml
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/frontmatter/position_parity_tests.rs
documentation:
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/inline/schema-validation.md
    - darkmatter/docs/errors/file-reference-failures.md
    - darkmatter/README.md
    - .claude/skills/darkmatter/schema.md
completed_phase: 4
implemented: true
---

# Implementation Log for 2026-10-15-diagnostic-node-positions (4 phases)

## Phase 1

Phase 1 is design and a one-time feasibility spike. It leaves no source change
behind: the spike was a temporary `#[cfg(test)]` probe module
(`darkmatter/lib/src/markdown/schemas/simplified/spike_probe.rs`, declared in
`simplified/mod.rs`) that was run with
`cargo nextest run --lib spike_probe_*` and then deleted, with the `mod`
line removed. `git status` confirms no Phase 1 residue under `darkmatter/`
(the modified `simplified/*.rs` files predate this phase and belong to the
branch's earlier union-type work).

### Contract inventory

**`PositionMap` sites** (Rust-scoped `rg -t rust`):

| Site | Use |
| --- | --- |
| `darkmatter/lib/src/markdown/schemas/validate.rs:45` | definition: `pub type PositionMap = IndexMap<String, (u32, u32)>` |
| `validate.rs::build_position_map` (~1234) | only builder: top-level line scan, column always 1 |
| `validate.rs::build_problem` (~744) | only lookup: `positions.get(identify_key(..))` |
| `validate.rs` `collect_problems*`, `collect_root_union_problems*`, `collect_arm_problems_with_anchors`, `narrow_discriminated_union_problems`, `build_problems` | pass-through `&PositionMap` |
| `schemas/mod.rs:145` | public re-export |
| `schemas/mod.rs::positions_for` (~1741) | the only production constructor; called by `DarkmatterSchemas::validate` and `validate_for_phase` (mod.rs ~711, ~740) |
| `schemas/mod.rs` `validate_with_positions`, `validate_for_phase_with_positions`, `validate_raw_with_positions`, `validate_instance`, `validate_with_options` | pass-through; source-free twins pass `PositionMap::new()` |
| `dmls/src/diagnostics/frontmatter.rs:77`, `frontmatter/severity_tests.rs:118,142` | `PositionMap::new()` only (DMLS ranges via its own `FrontmatterAst`) |
| `cli/src/commands/schema/validate.rs:523` | comment in `format_location` naming `build_position_map` (column-1 assumption) |
| tests: `validate.rs` (many `PositionMap::new()`, two mutable maps at ~1838/~1888, two `build_position_map` tests ~1895/~1905), `schemas/tests/mod.rs`, `simplified/convert.rs:1495,2405` | `PositionMap::new()` / direct insert |

No crate outside `darkmatter/` names `PositionMap` or `build_position_map`
(checked repository-wide, including `claudine/`). Iteration only happens in
`positions_for` (`iter_mut` to add the delimiter line). Direct `insert` occurs
only in `validate.rs` unit tests — Phase 2 must migrate those two tests.

**Locator consumers that are *not* map consumers:**
`markdown/hash/write.rs` (~623) and
`markdown/compose/frontmatter_interpolation.rs` (~527) call
`locate_frontmatter_value` directly and align by comparing decoded scalar
values. They do not touch `PositionMap`; leave `source.rs` untouched so their
behavior cannot change.

**Snapshot survival.** `Markdown.loaded: Option<Arc<LoadedSource>>` holds the
original bytes. It is set only at parse time (`From<String>`, `TryFrom<&Path>`,
`try_from_content` path at `markdown/mod.rs` ~367/~1034/~1069) and is never
cleared by `frontmatter_mut()`. Both mutation paths that precede validation
mutate in place: the CLI assignment pass
(`cli/src/commands/schema/validate.rs` → `assignment::apply_all(md.frontmatter_mut().as_map_mut(), ..)`)
and composition's coercion write-back
(`compose/schema_validation.rs` ~289, `frontmatter_mut().as_map_mut()`).
Composition's two validation passes (`schema_validation.rs` ~316 and
`validate_typed_shell_values` ~471) both call `DarkmatterSchemas::validate*`
on the same `Markdown`, so both reach `positions_for` with the original
snapshot. `Frontmatter::mapping_orders()` likewise survives mutation (no code
reassigns it after parse; `effective_state.rs:565` replaces a separate state
struct's field, not a `Markdown`'s).

**Source selection hazard found.** `From<String>` attaches `loaded` text even
when frontmatter parsing *failed* (the document then becomes body-only with an
empty map and `raw_source() == None`). Precise projection must therefore be
gated on `frontmatter().raw_source().is_some()`; otherwise a failed parse's
bytes would be indexed against an empty semantic map.

**Semantic-parse hazard found.** `parse_frontmatter` uses
`parse_yaml_with_fallbacks` (direct → tab normalization → `$(…)`/`{{ … }}`
placeholder protection). The semantic map can therefore come from text that
differs from the authored bytes, and the tolerant locator "does not require
every line to belong to the value". The spec's "never index unmatched nodes"
needs an explicit alignment check (see design below).

### Spike results (existing locator, unchanged)

`locate_frontmatter_value(yaml, offset)` already returns decoded keys, distinct
key spans, and value spans for every supported presentation. Coordinates below
are line:column within the YAML slice (add one line for a complete Markdown
document whose first line is `---`).

| Case | Result |
| --- | --- |
| acceptance (`start/then/[say, sya]`) | `sya` key 4:9 → **document 5:9**; item mapping start == first key start (dash excluded); value `goodbye` 4:14 |
| block list `tags: [1, oops]` | item scalar `oops` 3:5 (dash at col 3 excluded) |
| second item missing a field | item mapping starts at its first key, 4:5 |
| flow `m: {a: 1, b: x}` / `s: [1, two, 3]` | `b` key 1:11, `x` value 1:14; `two` 2:8; flow mapping value starts at `{` (1:4) |
| quoted keys `"0"`, `"a/b"`, `"a~b"`, `""`, `'q''s'`, `"esc\u0041"` | decoded keys `0`, `a/b`, `a~b`, empty, `q's`, `escA`; key span covers the quotes |
| block scalars `\|` / `>` | one scalar node starting at the indicator (1:6) |
| quoted JSON `cfg: '{"a": …}'` | one scalar at the opening quote |
| leading comments / blank lines | root mapping starts at first key (4:1) |
| CRLF | identical lines/columns, byte offsets shifted by the `\r`s; spans exact in original bytes |
| compact wide items `-   a: 1` | mapping start at `a` (2:7) |
| indentless `list:\n- a` | sequence at col 1, items at col 3 |
| anchor `base: &b\n  x: 1` | mapping span starts at the `&b` token |
| tag `!!str 5` | scalar span starts at the tag |
| empty (null) value `a:` | zero-width scalar right after the colon |
| multi-line plain / quoted scalar | one scalar node starting at its first token |
| explicit key `? a` | mapping keyed `a`, starting at the key |
| non-ASCII (`"😀"` key in flow) | byte columns (key at byte col 11 in `x: {k: 1, "😀": 2}`) |
| trailing comment `a: 1 # c` | scalar start correct (span end includes the comment — only starts matter here) |
| `$(echo hi)` plain, `"{{ x }}"` quoted | located as scalars |
| unquoted `a: {{ x }}`, flow spread over lines, comment inside flow | **`None`** for the whole block → legacy fallback |
| alias `other: *b` | located as **scalar** while the semantic value is a mapping |
| merge key `<<: *b` | located `<<` key with **scalar** value; semantic `<<` is a mapping (serde_yaml_ng resolves aliases but does not apply merges) |
| BOM + CRLF complete document | `extract_frontmatter_block` yaml_span starts after the BOM line; spans exact |
| tabs (`a:\n\tb: 1`) | locator accepts; semantic parse succeeds via tab normalization; same structure |
| unquoted non-string keys (`1: a`, `true: b`) | frontmatter parse fails entirely → body-only document, `raw_source() == None`, empty map; never validated with positions |

No locator adjustment is needed. Unsupported presentations degrade to `None`
for the whole block or are caught by the alignment witness below.

### Chosen design (binding for Phase 2)

**Source.** In `positions_for`, when `raw_source().is_some()` and
`loaded_source()` is `Some`, run `extract_frontmatter_block(&loaded.text)` and
call `locate_frontmatter_value(block.yaml, block.yaml_span.start)` so every
span is a byte offset in the *complete loaded document*. Build the line-start
index over `&loaded.text[..block.block_span.end]` (frontmatter block only, not
the body). Line numbers are then document lines directly, so the delimiter
offset is applied exactly once by construction, for key, value, root, and
fallback positions alike. Without loaded text, use `raw_source()` with base
line 2 (today's `LEADING_DELIMITER_LINES` behavior); without raw source, use
the serialized view (base line 1) as today. `Frontmatter::raw_source()` must not
be used for byte offsets when loaded text exists (it re-joins lines with LF).

**Alignment witness (new, cheap, no re-parse).** Accept the located tree only
if it matches `Frontmatter::mapping_orders()` — the authored key order of every
mapping, collected from the successful semantic YAML parse at load time and
untouched by later mutation:

1. every located mapping at pointer `P` has decoded keys equal (in order) to
   `orders.get(P)`;
2. the number of located mappings equals the number of indexed mappings (so a
   semantic mapping the locator saw as a scalar — alias, merge source — fails);
3. no located scalar is an alias token (`*name`; reuse
   `simplified::yaml_scalar::alias_token`), since sequences are not indexed by
   `MappingOrders` and an alias to a sequence would otherwise slip through.

On failure, discard the precise tree and use the legacy top-level scan. For
the serialized view, collect the witness from the `serde_yaml_ng::Value` that
is about to be serialized (`MappingOrders::collect(&yaml_value)`). The public
low-level `build_position_map(yaml)` has no `Frontmatter`; it parses `yaml`
once with `serde_yaml_ng` to obtain the witness (once per call, never per
problem) and falls back to the legacy scan when that parse fails.

**Representation.** Replace the type alias with a struct (public, opaque
fields), e.g.:

```rust
pub struct PositionMap {
    source: PositionSource,
}
enum PositionSource {
    /// Source-free validation.
    Empty,
    /// Located, witness-matched authored tree.
    Located { root: SchemaValueNode, lines: LineIndex },
    /// Legacy top-level key → (line, 1); projection unavailable.
    TopLevel(IndexMap<String, (u32, u32)>),
}
struct LineIndex { starts: Vec<usize>, base_line: u32 }
```

`PositionMap::new()` (and `Default`) stays the empty map. `LineIndex` converts
a byte offset to `(line, byte_col)` by binary search; `base_line` is the line
number of byte 0 of the indexed text (1 for document text and for a bare YAML
slice; 2 for the raw-source fallback, 1 for the serialized view). The two
`validate.rs` unit tests that `insert` into a map must be rewritten against
`build_position_map` input instead.

**Lookup.** One method taking the problem's category, decoded pointer
segments (`JsonPointer::segments()`), and `offending_property`:

- value failures → the located node at the pointer (its `span.start`);
- `UnknownKey` → key span of `offending_property` in the mapping at the
  pointer;
- `MissingRequired` → the mapping at the pointer (the parent; for root that is
  the root mapping start);
- root value failures → root start.

Walk segments by node kind: mapping → find entry by decoded key (a segment
`"0"` is a key here); sequence → segment must parse as an index; scalar →
stop and use the scalar start for every descendant (decoded block/quoted
content). If the walk falls off, use the deepest located *non-root* node
reached; if only the root was reached, use the top-level key span for the
first segment if authored; else no position. In `TopLevel` mode, look up the
first pointer segment only (the parent's first segment for unknown/missing
keys); root-level unknown keys use `offending_property` as the top-level key
there too. Note: today's `identify_key` uses a *nested* missing property's own
name as a top-level key, which can match an unrelated top-level key; the new
selection uses the parent pointer instead — a correctness improvement within
the spec's fallback chain, not a policy change.

**Snapshot reuse.** `positions_for` already runs once per
`DarkmatterSchemas::validate*` call, outside problem and union-arm loops; all
`collect_*` functions receive `&PositionMap`. Keeping projection inside
`positions_for`/`build_position_map` preserves that structurally — no
per-problem projection.

### Fixture coordinates for Phases 2–3 (complete Markdown, LF, first line `---`)

| Fixture | Expected |
| --- | --- |
| acceptance (spec criterion 1) | unknown key `sya`: 5:9, pointer `/start/0/then/1` |
| `---\ntags:\n  - 1\n  - oops\n---` | `/tags/1` type error: 4:5 |
| `items:\n  - name: a\n    id: 1\n  - name: b` | `/items/1` missing `id`: 5:5 |
| `m: {a: 1, b: x}` on line 2 | unknown `b`: 2:11; wrong value `/m/b`: 2:14 |
| `s: [1, two, 3]` on line 3 | `/s/1`: 3:8 |
| `# comment\n\n# again\ntitle: x` | root missing field: 5:1 |
| `doc: \|` on line 2 | `/doc` and any `/doc/...` descendant: 2:6 |
| CRLF acceptance | 5:9; `sya` byte offset 52 (opening `---\r\n` is 5 bytes) |

### Notes and residual risks for later phases

- Precision is lost for the whole document when any unquoted `{{ … }}` or a
  multi-line flow collection appears (locator returns `None`). That is the
  specified best-effort fallback, but it is common in composition documents;
  tests should assert the legacy top-level fallback there rather than assume
  precision.
- A trailing comment is included in a plain scalar's span end. Starts are
  unaffected; DMLS ranges are out of scope.
- Null values (`a:`) locate as a zero-width scalar immediately after the
  colon; a type error on such a value points there.
- DMLS parity (Phase 3) compares against `FrontmatterAst::value_range` /
  `key_span_for` / `parent_mapping_range` starts; anchors and tags (where the
  locator's start includes the `&b`/`!!str` token) are outside the "supported
  nodes" parity set unless DMLS agrees.

### Verification

- Spike probes ran green (`cargo nextest run --lib spike_probe_dump`,
  `spike_probe_edges`) and were then deleted.
- No source, doc, or skill file changed in this phase; the claudine skill
  needed no update.
- Phase gates on macOS (this host), run in `claudine/`: `just lint` passed;
  `just test` passed (8684 tests run, 8684 passed, 2 slow, 9 skipped). No OS
  cross-check was needed because nothing compiled changed.
- Requirement-to-test mapping: none for Phase 1, which changes no behavior.
  The fixture coordinates above are the planned targets for the Phase 2–3
  tests.

## Phase 2

Phase 2 implements the authored position index (Wave 2) and category-aware
problem selection (Wave 3), following the Phase 1 binding design without
departures except where noted below.

### What changed

- **New module `darkmatter/lib/src/markdown/schemas/positions.rs`.**
  `PositionMap` is now a struct (opaque `PositionSource`: `Empty`,
  `Located { root: SchemaValueNode, lines: LineIndex }`, `TopLevel(IndexMap)`)
  instead of `IndexMap<String, (u32, u32)>`. `validate.rs` re-exports
  `PositionMap` and `build_position_map`, so `validate::build_position_map`
  and `schemas::PositionMap` paths are unchanged. Public surface:
  `PositionMap::new()`/`Default` (empty, source-free), `is_empty()`, and
  `build_position_map(yaml)`. Projection and lookup stay `pub(super)`.
- **Projection** (`PositionMap::project(text, yaml_range, base_line, orders)`)
  calls `locate_frontmatter_value` once, requires a mapping root, and accepts
  the tree only when the alignment witness holds: every located mapping's
  decoded keys equal `MappingOrders` at its pointer, in order; the located
  mapping count equals `orders.len()`; and no located scalar is an alias
  token. The `LineIndex` treats `\n`, `\r\n`, and a lone `\r` as one break and
  reports 1-based UTF-8 byte columns. Unprojectable YAML falls back to the
  legacy top-level scan (`PositionMap::top_level(yaml, base_line)`, column 1).
- **`MappingOrders::len()`** was added (public, `#[must_use]`) for the
  witness's mapping-count check.
- **`positions_for`** (`schemas/mod.rs`) projects the loaded document bytes
  (`extract_frontmatter_block` + `yaml_span`, base line 1, so the delimiter
  offset is applied once by construction) only when `raw_source()` is present;
  without loaded text it projects `raw_source()` at base line 2; on failure it
  uses the top-level scan of `raw_source()` at base line 2 (today's behavior).
  The serialized view for constructed documents is projected with
  `MappingOrders::collect` of the value being serialized, falling back to the
  top-level scan.
- **`build_problem`** (`validate.rs`) selects a `PositionTarget` from the
  typed error: `Required` → `Mapping` at the pointer; an error with an
  `offending_property` → `Key(property)` in the mapping at the pointer;
  anything else → `Value`. Lookup walks `JsonPointer::segments()` by node
  kind (sequences parse indices; mappings match decoded keys; a scalar stops
  the walk and supplies its start for any descendant). Off-tree paths use the
  deepest located non-root node, else no position; a new top-level property
  has none. In top-level mode the first pointer segment (or the unknown key
  at the root) is looked up. Projection still happens once per
  `DarkmatterSchemas::validate*` call in `positions_for`, outside problem and
  union-arm loops; `build_problem` only reads the shared map.
- **Comments.** `collect_problems` and `error_top_level_key` docs no longer
  claim that problem positions use top-level attribution (`identify_key` is
  now used only by `error_top_level_key`, for coercion/rewrite attribution,
  unchanged). `positions_for` gained a doc describing the source order.

### Behavior changes observed in existing tests

These are the intended position changes; every non-position field stayed
identical.

- `darkmatter/lib/tests/l1/meta_schema_phase4.rs`: a value failure on
  `candidate: string(min(nope))` now reports column 12 (the value) instead of
  column 1 (the key).
- `darkmatter/cli/tests/fixtures/schema_validate_baseline/*/expected.{json,pretty}`
  regenerated for five cases. A script compared old and new JSON with `line`
  and `column` removed: equal in all eight cases. Changes:
  `enum_member_invalid` /status 4:1→4:9; `missing_required` root missing
  field none→2:1 (root mapping start, the `$schema` key);
  `property_union_invalid` /count 6:1→7:3 (the block sequence start);
  `property_union_nested_object` /event/a 6:1→7:6 and /event/b 6:1→8:6;
  `root_union_none_match` root missing field none→2:1. The pretty snapshots
  still use the existing `(at line N of frontmatter)` wording, which Phase 3's
  `format_location` change will replace.

### Requirement-to-test mapping

Unit tests in `darkmatter/lib/src/markdown/schemas/positions.rs` (`tests`):

| Requirement | Test |
| --- | --- |
| slice-relative vs document-relative lines (acceptance 4:9 vs 5:9) | `nested_key_is_located_relative_to_the_indexed_text` |
| CRLF original-byte offsets (`sya` at byte 52, 5:9) | `crlf_offsets_address_the_original_bytes` |
| key vs value starts; flow mapping/sequence columns | `keys_and_values_have_distinct_starts_in_flow_collections` |
| list item excludes dash; second item mapping start | `block_list_items_exclude_the_dash` |
| root mapping start after comments/blank lines | `the_root_mapping_starts_at_its_first_key` |
| `"0"`, `"a/b"`, `"a~b"`, `""` keys; mapping vs sequence | `decoded_keys_and_indices_follow_the_node_kind` |
| block/quoted scalar descendants use scalar start | `decoded_scalar_descendants_use_the_scalar_start` |
| fallback chain (ancestor / none for new top-level) | `unauthored_paths_fall_back_to_the_nearest_authored_non_root_node` |
| UTF-8 byte columns | `columns_count_utf8_bytes` |
| aliases (mapping and sequence), merge keys, unquoted templates → top-level fallback | `unsupported_yaml_keeps_the_top_level_fallback` |
| witness rejects mismatched structure / empty orders | `a_mismatched_structure_is_not_projected` |
| empty map yields no positions | `the_empty_map_has_no_positions` |
| colon-in-key top-level scan (migrated) | `top_level_keys_keep_a_colon_inside_a_key` |
| legacy base-line offset | `top_level_fallback_offsets_lines_by_its_base` |

Public-path tests in `darkmatter/lib/src/markdown/schemas/tests/positions.rs`
validate complete Markdown through `DarkmatterSchemas::validate` with a
baseline schema, and every case also asserts that the source-free report
(`EffectiveSchema::validate`) has the same validity and the same problems, in
the same order, field for field (via `Debug`), once `line`/`column` are
removed:

| Requirement | Test |
| --- | --- |
| acceptance fixture, LF and CRLF: pointer, `offending_property`, 5:9 | `nested_unknown_key_reports_its_key_token` |
| type error in block list → `oops` 4:5 | `a_wrong_list_item_reports_its_value_without_the_dash` |
| missing field in second item → mapping 5:5 | `a_missing_field_reports_the_mapping_that_lacks_it` |
| flow mapping unknown key 3:11, value 3:14; flow sequence 4:8 | `flow_collections_distinguish_same_line_keys_and_values` |
| root missing field → root mapping start | `a_root_failure_reports_the_root_mapping_start` |
| constraint and file-reference failures → value start | `constraint_and_file_reference_failures_report_their_values` |
| multi-key unknown error → one problem at the selected key | `a_multi_key_unknown_error_locates_its_selected_property` |
| property union unchanged and located | `a_property_union_keeps_its_problems_and_locates_them` |
| block scalar → scalar start | `block_scalar_content_reports_the_scalar_start` |
| replaced path → original node; new nested → ancestor; new top-level → none | `replaced_and_new_paths_use_the_original_source` |
| valid alias stays valid; independent failure keeps top-level fallback | `unsupported_yaml_keeps_validity_and_top_level_positions` |
| constructed document → serialized view | `a_constructed_document_uses_its_serialized_view` |
| failed frontmatter parse with loaded text → no positions | `unparsed_frontmatter_reports_no_positions` |

In `validate.rs` tests: `root_union_problems_are_located_in_the_shared_map`
(one map shared across root-union arms; arm selection unchanged), and the two
tests that inserted into the old map now build their map with
`build_position_map` (`collect_problems_populates_line_for_type_mismatch`,
`collect_problems_populates_line_for_missing_required`). The old
`build_position_map_*` map-`get` tests moved into `positions.rs`.

All new tests are in the `darkmatter` lib unit-test binary (L1, no marker
prefixes, no filesystem fixtures beyond `std::env::temp_dir()` as an unused
resolution `cwd`, no process spawns).

### Gates (macOS, this host)

- `darkmatter/`: `just test` — 9487 passed, 11 skipped; `just lint` — passed,
  including `check-zed` (`wasm32-wasip2` is installed).
- `claudine/`: `just test` — 8684 passed, 9 skipped; `just lint` — exit 0 (the
  only warning is the existing macOS linker `__eh_frame` notice).
- No other OS was exercised: the change is pure string/byte processing with no
  `cfg`, path, or process code, and the tests use in-memory strings. CI's
  Linux/macOS legs cover it.
- The Input Robustness Matrix does not apply (no reader, deserializer, or
  acceptance policy changed), as recorded in the plan.

### Notes for Phase 3

- `darkmatter/cli/src/commands/schema/validate.rs::format_location` still
  prints `(at line N of frontmatter)` and its comment names the column-1
  assumption; both are now wrong. Updating it will change the
  `schema_validate_baseline` `expected.pretty` snapshots again.
- A property union of two inline objects discriminated by `enum(...)` reports
  `"bar" is not one of "foo"` at `/payload/type` and no unknown-key problem.
  That is existing union reporting, unchanged by this phase.

## Phase 3

Phase 3 covers Wave 4 (CLI presentation, DMLS parity) and Wave 5 (regression
coverage, behavior documentation). The only production change is the CLI's
human-output location text. Everything else is tests and docs. The library's
position index and DMLS's range rules are unchanged.

### What changed

- **CLI human output.** `format_location` in
  `darkmatter/cli/src/commands/schema/validate.rs` now prints
  `line L, column C`, or `line L` when the column is absent, and nothing when
  there is no position. The obsolete "always column 1" comment and the
  `of frontmatter` wording are gone; the line counts the complete document.
  The five failing `schema_validate_baseline/*/expected.pretty` snapshots
  changed only in that location text (`(at line 4 of frontmatter)` →
  `(at line 4, column 9)` and so on). Their JSON snapshots were regenerated
  in Phase 2 and did not change here. The existing test
  `schema_validate_pretty_reports_line_for_type_mismatch` was renamed
  `..._line_and_column_...` and now asserts `(at line 4, column 9)`. Its
  comment about "re-serialised frontmatter" was stale and has been corrected.
- **Compose error block.** `errors/blocks.rs::schema_validation_block`
  already renders ` at L:C` and has no excerpt or caret. A nested-coordinate
  test passes unchanged, so the block was not modified. Its
  `(Some(line), None) => ":1"` branch was left alone: the library never
  produces a line without a column.
- **Shared fixture.** `darkmatter/lib/tests/position_fixtures/mod.rs` is
  std-only and holds complete Markdown bytes plus `ExpectedStart`
  (path, kind, offending property, line, column), together with `CASES` and
  `byte_offset`. Cases: `ACCEPTANCE`, `BLOCK_LIST`, `FLOW`,
  `NON_ASCII_PREFIX`, and `ROOT_MISSING`. Each document declares its
  `$schema` *after* the authored properties, so the acceptance lines are
  byte-identical to the spec's example. The darkmatter lib (`tests/l1`), the
  CLI (`tests/l1`), and DMLS (unit tests) include it with `#[path]`. The cli
  and dmls `Cargo.toml` list it under `source-inputs`, and every test that
  uses it spells it with `include_str!`.

### DMLS parity finding

DMLS's diagnostic start equals the library's byte offset for every fixture
problem, in both `PositionEncoding::Utf8` and `Utf16`, with LF and CRLF.
With the default configuration, DMLS emits **no** diagnostic for
missing-field problems: `classify` hides `MissingRequired` unless
`schema.strict` is on. That is existing behavior and was left unchanged. The
parity tests therefore check missing-field problems (the second item mapping
and the root mapping) under strict mode, and values and unknown keys under
both modes. The DMLS ranges recorded as unchanged literals are: `sya`,
`oops`, `name: b\n` (item mapping), `b`, `x`, `two`, `zz`, `x`, and the root
mapping through the last frontmatter line.

### Requirement-to-test mapping (Phase 3 additions)

| Requirement | Test |
| --- | --- |
| Acceptance 5:9 in CLI JSON (LF, CRLF) | `darkmatter-cli::l1 diagnostic_positions::json_reports_the_acceptance_key_token`, `..._with_crlf_line_ends` |
| Acceptance 5:9 in human output, exit 1 | `diagnostic_positions::human_output_shows_line_and_column` |
| Acceptance 5:9 in `md compose` error block, exit 1 | `diagnostic_positions::compose_error_block_shows_line_and_column` |
| Block-list value excluding dash; second-item mapping | `diagnostic_positions::json_reports_block_list_items_and_mappings_without_the_dash` |
| Flow key vs value same-line columns | `diagnostic_positions::json_distinguishes_same_line_flow_keys_and_values` |
| UTF-8 byte columns after non-ASCII prefix | `diagnostic_positions::json_columns_count_utf8_bytes_after_a_non_ascii_prefix` |
| Root missing → root mapping start | `diagnostic_positions::json_reports_the_root_mapping_for_a_root_missing_field` |
| Replaced path → original node; new top-level → JSON `null` | `diagnostic_positions::assigned_values_use_the_original_node_or_report_null` |
| Success exit 0, empty problems | `diagnostic_positions::a_valid_document_exits_zero_with_no_problems` |
| Guard: every shared case has a CLI test | `diagnostic_positions::every_shared_case_has_a_json_test` |
| Library positions and `offending_property` for every shared case, LF and CRLF | `darkmatter::l1 diagnostic_positions::every_shared_case_reports_its_expected_starts` |
| Fixture starts address authored tokens; byte offsets 48 / 52 (CRLF) | `diagnostic_positions::expected_starts_address_the_authored_tokens` |
| DMLS start = library byte offset (UTF-8, UTF-16), plus unchanged range end and ranged text | `dmls diagnostics::frontmatter::position_parity_tests::dmls_starts_match_library_starts_in_utf8`, `..._in_utf16` |
| DMLS parity with CRLF | `position_parity_tests::dmls_starts_match_library_starts_with_crlf_line_ends` |
| UTF-8 vs UTF-16 columns actually differ (conversion is load-bearing) | `position_parity_tests::non_ascii_prefix_columns_differ_between_encodings` |
| Quoted/escaped keys `"0"`, `"a/b"`, `"a~b"`, `"escA"`; pointer `~1`/`~0`; mapping vs sequence | `darkmatter` lib `schemas::tests::positions::quoted_keys_are_matched_decoded_and_escaped_in_the_pointer` |
| Empty key `""` is a property, not the root | `positions::an_empty_key_is_a_property_not_the_root` |
| Literal/folded/quoted-JSON scalars and in-memory decoded descendants → scalar start | `positions::scalar_content_and_its_descendants_report_the_authored_scalar_start` |
| BOM + CRLF + leading comments/blank line → one delimiter offset | `positions::a_bom_crlf_document_with_leading_comments_counts_lines_once` |
| Root union nested failure, arm selection, field equality | `positions::a_root_union_locates_a_nested_failure_in_its_selected_arm` |
| Shipped schema end-to-end (`content-policy/schemas/content-policy.yaml`) | `darkmatter::l1 content_policy_editor_schema::shipped_schema_problems_report_their_authored_positions` |

The remaining rows of the plan's regression table were already covered by the
Phase 2 tests listed under `## Phase 2`. The shared-locator consumers are
covered by the existing hash and interpolation suites, which still pass
(593/593 for `test(hash) | test(interpolation)`). The shipped-schema corpus
tests `shipped_schema_corpus` and `meta_schema_repo_schemas` still pass.

Finding: no public path decodes a quoted JSON or YAML string into an object.
`json` and `yaml` are string-format types, and coercion only converts native
values into strings. A "decoded descendant" is therefore reachable only by
mutating the map in memory, and the scalar test covers it that way.

### Documentation

- `darkmatter/docs/topics/schemas/definition.md` has a new `### Problem Locations`
  section: the nested example, how coordinates are counted, which node each
  problem kind points at, flow columns, scalars, a fallback Mermaid diagram,
  constructed and source-free documents, and `build_position_map`. The
  Pretty, JSON, and assignment sections were corrected; the stale "no
  line/column inside a list item" limitation was removed; and a `PositionMap`
  key-type row was added. The assignment example (`count=many` → 2:8,
  `user.age=x` → 4:3, `rating=x` → none) was checked with the real `md`
  binary. The Documenter also corrected the vacuous-success message to the
  text the CLI actually prints.
- `darkmatter/docs/inline/schema-validation.md` and
  `docs/errors/file-reference-failures.md` now use the real location format.
- `darkmatter/README.md` has a one-bullet feature note. Its `hash:` was
  already stale at HEAD (body hash mismatch before this change) and was not
  re-stamped.
- `.claude/skills/darkmatter/schema.md` has a "Problem positions" block, and
  its stale "Known gaps: item problems carry the top-level key's line/column"
  line was removed.

### Gates (macOS, this host)

- `darkmatter/`: `just test` — 9510 passed (1 slow), 11 skipped; `just lint` —
  exit 0, including `check-zed`.
- `claudine/`: `just test` — 8684 passed, 9 skipped; `just lint` — exit 0.
- No other OS was exercised. The changes use in-memory strings and fixture
  temp files only, with no `cfg`, path-comparison, or process-specific code.
  CRLF is tested explicitly. Windows and WSL2 results are not claimed.
- No test uses a tier marker prefix, and all new tests are hermetic L1 in
  declared targets. The cli and lib layout gates pass.
- The claudine skill needed no update; nothing about Claudine changed.

### Pre-existing / skipped

- No failures. The 11 darkmatter and 9 claudine skips are the suites' existing
  skips.

## Phase 4

Phase 4 is verification and review handoff. It changed no source, docs, or
skills. Only the plan, this log, and the spec frontmatter were edited.

### Package verification (macOS, this host, 2026-10-12)

- `darkmatter/`: `just test` — **9510 passed, 11 skipped** (lib, CLI, DMLS,
  zed-dmls-cli). `just lint` — **exit 0**, including `check-zed`; the
  `wasm32-wasip2` target is installed (`rustup target list --installed`), so
  this is not an environment blocker.
- Downstream consumers: `rg -t rust 'PositionMap|build_position_map'` finds
  no use outside `darkmatter/`, which agrees with the Phase 1 inventory.
  `claudine` depends on darkmatter and was compiled and tested as the
  downstream check.
- `claudine/`: `just test`. The first run hit one failure,
  `claudine-cli::l1 wrap_perf::composition_setup_and_provider_handoff_order_matches_phase_1_baseline`,
  and nextest stopped at 6207/8684. That output was piped through a filter,
  so the assertion message was not captured. Run alone, the test passed 3 of
  3 times, and a full rerun passed **8684, with 9 skipped**. The test composes
  a schema-valid document with a `goose` stub and checks stdout ordering. It
  has no schema-validation problem, so no location code runs. It is treated
  as an intermittent failure under load, unrelated to this feature, and is
  recorded here for the reviewer. `just lint` — **exit 0**.
- Other OSes were not run. As in Phases 2 and 3, the change is in-memory
  byte/string processing with no `cfg`, path-comparison, or process code, and
  CRLF and non-ASCII columns are tested explicitly. Linux, Windows, and WSL2
  results are not claimed; CI's Linux and macOS PR legs will cover the change.

### Review handoff

Acceptance criteria against evidence (the test names are in the Phase 2 and
Phase 3 mapping tables):

1. Nested `sya` at `/start/0/then/1`, `offending_property: sya`, 5:9 in
   JSON and human output: `darkmatter-cli::l1 diagnostic_positions::json_reports_the_acceptance_key_token`
   (+ CRLF), `human_output_shows_line_and_column`.
2. Block-list value without the dash, second-item mapping, key versus value,
   flow mapping and flow sequence: `json_reports_block_list_items_and_mappings_without_the_dash`,
   `json_distinguishes_same_line_flow_keys_and_values`, and the lib tests.
3. `md compose` error block 5:9: `compose_error_block_shows_line_and_column`.
4. DMLS parity in UTF-8 and UTF-16, with ASCII and a supplementary-plane
   prefix, comparing byte offsets and checking that ranges are unchanged:
   `dmls position_parity_tests::*`.
5. Assignment, new nested and new top-level paths, root missing field, block
   scalar, decoded descendant, unsupported YAML, and empty map: Phase 2 lib
   tests plus `assigned_values_use_the_original_node_or_report_null`.
6. Quoted, escaped, empty, and numeric keys; LF/CRLF; comments; delimiter
   offset; field equality; locator consumers: `positions::*` tests, the
   hash and interpolation suites.
7. `just test` and `just lint` pass in `darkmatter/` (above); all tests are
   L1, CLI spawns use `CliProcessFixture`, there are no windows, and the CI
   configuration is unchanged. None of this feature's commits touches
   `.github/`, a justfile, or the nextest configuration.

- Snapshot reuse: one projection per validation source snapshot, shared
  across problems and union arms (Phase 2,
  `root_union_problems_are_located_in_the_shared_map`).
- The diagnostic and exit-code contract is unchanged except for
  `line`/`column` (Phase 2 field-equality tests and the CLI exit-code tests).
- Specification departures: none beyond those already recorded in Phases 2
  and 3, namely DMLS missing-field parity is checked only in strict mode,
  and no public path decodes a quoted JSON or YAML scalar into an object.
- The claudine skill needed no update; Claudine's behavior did not change.

Implementation is complete and ready for review. The feature stays in its
active directory.
