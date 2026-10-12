---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/claudine/features/2026-10-15-diagnostic-node-positions/spec.md"
plan: "claudine/features/2026-10-15-diagnostic-node-positions/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
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
