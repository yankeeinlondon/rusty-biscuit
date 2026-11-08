---
total_phases: 3
created: 2026-11-08
phase: 1
agent: claude/sonnet
yolo: true
packages:
- simplified-schema
- darkmatter
- darkmatter-cli
- darkmatter-lsp
---

# YAML Locator Hardening: Implementation Plan

Spec: `2026-11-05-yaml-locator-hardening`. Non-interactive execution: no phase
waits for a human, every choice is already written down in
[Necessary Rules](#necessary-rules). Sub-agents inherit that the whole session
tree is non-interactive. Tests must never open or focus a terminal or browser
window. Run tests through `just test` (nextest), never `cargo test`; never run
`cargo fmt`; never commit.

## Summary and Definition of Done

### What is wrong today

A YAML data file or frontmatter block that contains **any** alias (`*name`)
loses every nested position, and `load_data` in Darkmatter additionally
discards positions for any multi-line flow collection. Two code sites cause it:

1. `simplified-schema/lib/src/positions.rs`: `matches_orders` returns `false`
   for any scalar that `alias_token` recognizes, and `PositionMap::project`
   additionally demands `mappings == orders.len()`. The parsed value expands
   aliases, so the order index (`MappingOrders`) holds mappings below an alias
   that have no authored text. `project` returns `None`, and
   `build_position_map` / `positions_for` fall back to `PositionMap::top_level`
   (column-1 guesses; `byte_range` returns `None`).
2. `darkmatter/lib/src/markdown/compose/expression/data/locate.rs`:
   `has_multi_line_flow_collection` deliberately drops the map for a multi-line
   flow collection even though the shared locator supports one.

I verified at planning time (scratch probe test, reverted) that the shared
tolerant locator `locate_frontmatter_value` already builds correct authored
trees for the alias shapes in the spec: an alias in a block value, a block
sequence item, a flow sequence item, and a flow mapping value (including
`{<<: *b, port: 2}`) is a `Scalar` node whose span is the alias text, an
anchored original (`base: &b {port: x}`) is a `Mapping` node whose span starts
at the anchor, and a multi-line flow sequence with a comment containing `]` and
a quoted `"]"` locates every item. So **no change to the locator in
`simplified/source.rs` is planned**; the work is projection verification
(simplified-schema) and removing the suppression (Darkmatter).

### What changes

- `PositionMap::project` accepts an alias as an opaque leaf: the alias token
  `*name` becomes the leaf's span, its pointer becomes an *alias boundary*, and
  every indexed mapping at or below a boundary is accounted for without an
  authored node. Unrelated key or count mismatches still refuse projection.
- `SourceLocator` in Darkmatter stops suppressing multi-line flow positions.
- Docs, comments, skill text, and tests stop claiming that an alias or a
  multi-line flow collection makes a file pointer-only.

### Definition of Done

1. The two acceptance examples in the spec report exactly: `/servers/1/port` at
   line 4, column 12; `/base/port` at 1:17, `/one/port` at 2:6, `/two/port` at
   3:6, with byte ranges covering only `x` / `*b`.
2. Aliases to scalar, sequence, and mapping values, a sequence root, repeated
   uses, escaped pointer keys (`~`, `/`), and literal `<<` entries (bare,
   quoted, and sequence-of-aliases) keep neighbor positions; the
   `/a` boundary does not hide a mismatch at `/ab`.
3. Multi-line flow sequences of objects and nested flow mappings report
   positions inside and after the collection, with an alias item, comments, and
   quoted `]`/`}`.
4. `load_data` records, frontmatter problems, whole-file YAML validation, and
   DMLS ranges all carry the improved locations; counts, pointers, order,
   limits, messages, codes, malformed-input locations, and the CLI `source`
   field are unchanged.
5. `just test` and `just lint` pass in `simplified-schema/` and `darkmatter/`;
   no test or doc still claims an alias or multi-line flow forces a
   pointer-only file; the implementation log records the intentional
   replacement of the pointer-only exception in `2026-11-04-loading-data`.

### Input robustness matrix

Not applicable as a field matrix: this change adds no config field or
deserializer. The only loader in play (`load_admitted_yaml` plus the position
projection) keeps its accepted documents, duplicate-key, second-document, and
undefined-alias behavior; those malformed shapes are non-goals and their
existing tests (`load_data_diagnostics.rs` lines near 723-745,
`load_data_execution.rs:399`) are left untouched and must stay green. The
alias and flow *document shapes* the projection must handle are enumerated as
test cases in Phases 1 and 2. Smell grep, done at planning time: the touched
code has no `#[serde(default)]`, `filter_map(.. as_str())`, or
`unwrap_or_default()` on a load-bearing parse.

## Phase 1: Shared alias projection (simplified-schema)

Goal: `PositionMap::project` locates documents containing aliases, with exact
alias-token ranges, and keeps refusing genuine mismatches.

### Necessary Rules

Each rule states the decision, then the evidence.

1. **Alias detection uses only `alias_token(text, node.span.start)` on a
   `Scalar` node.** Evidence: `matches_orders` already does this
   (`positions.rs:296`); the locator only produces `Scalar` nodes for
   plain/quoted/block scalars and aliases, so a `*name` inside a comment, a
   quoted string, or a block scalar is never a node whose span starts with `*`.
   No raw-text search for `*` is added.
2. **An alias leaf's span is normalized to `alias_token`'s range.** `project`
   rewrites `node.span` of each alias scalar to the token range, so
   `byte_range` covers only `*name` (no trailing comment or delimiter). The
   probe showed the locator already yields exactly the token (`24..26` for
   `one: *b # c`), so this is a guarantee, not a repair. Therefore
   `matches_orders` takes `&mut SchemaValueNode`.
3. **Alias boundaries are a `HashSet<String>` of escaped JSON Pointers** built
   with the same `with_child` / `MappingOrders::child_pointer` path that builds
   lookup pointers. Evidence: `child_pointer` escapes `~`→`~0` and `/`→`~1`, so
   `/` occurs in a pointer only as a segment separator and prefix tests at `/`
   boundaries are decoded-segment-correct (`/a` never covers `/ab`;
   `/a~1b` never covers `/a`).
4. **Completeness check becomes `mappings + covered == orders.len()`.**
   `mappings` is the count of authored mappings, each already required to
   exist in `orders` at its pointer with identical ordered keys (so distinct
   from covered ones: an authored mapping never lies below an alias leaf).
   `covered` is the number of `orders` pointers equal to, or below, a
   boundary. `covered` is computed in **one pass over `orders`** (never per
   alias): for each pointer test the whole pointer and each prefix ending
   before a `/` (skipping the empty prefix) against the set. When the set is
   empty the pass is skipped and the check is exactly `mappings ==
   orders.len()` as today. This needs a new `pub(crate) fn pointers(&self) ->
   impl Iterator<Item = &str>` on `MappingOrders`
   (`simplified-schema/lib/src/mapping_orders.rs`).
5. **Merge keys stay literal; no code is added for them.** `serde_yaml_ng`'s
   `from_str::<Value>` does not apply `<<`, so `orders` holds `/server/<<`
   (the expanded alias) as an ordinary key; the locator yields a `<<` key with
   an alias leaf (probe confirmed), so `/server/<</port` resolves through the
   alias leaf to `*b`. A sequence of aliases (`<<: [*a, *b]`) produces
   boundaries `/server/<</0` and `/server/<</1` by the same mechanism.
6. **`locate_range_in` / `locate_in` / `byte_range` / `locate_problem` are
   unchanged.** Evidence: the loop already returns `node.span` when it meets a
   `Scalar` mid-path (`positions.rs:250`) and the `Key`/`Mapping` targets on a
   `Scalar` return `node.span` (lines 279, 281), which is exactly "a failure on
   the alias or any expanded descendant, an unknown key under an alias, or a
   missing child of an alias, points to the alias token".
7. **The fallback stays.** `build_position_map` still falls back to
   `PositionMap::top_level` when `project` is `None`. The fixture for "an
   unrelated mismatch still refuses projection" is `"&k a: 1\nb: 2\n"` (an
   anchored key): probe showed `build_position_map` leaves it un-located today,
   it contains no alias, and this change touches only alias handling.
   `"a: *b\n"` (undefined alias; the parse fails first) also stays un-located.
8. **Existing tests that encode the old behavior are rewritten, not deleted
   blindly.** In `positions.rs` `mod tests`:
   `unsupported_yaml_keeps_the_top_level_fallback` (its four fixtures: the
   `title: x\nother: {{ value }}\n` template fixture keeps its refusal; the
   three alias/`<<` fixtures move to the new located tests) and
   `an_alias_under_a_list_root_still_refuses_projection` (becomes
   `an_alias_under_a_list_root_is_projected`).
9. **Doc comments to fix in this phase** (code is correct, comments drift):
   `PositionMap` type doc, `PositionMap::project` `## Returns` ("contains an
   alias" clause), `matches_orders` doc ("and no scalar is an alias"), the
   `PositionSource::Located` comment, and the `build_position_map` doc (add
   that aliases are projected). Also `PositionMap::byte_range` doc gains one
   sentence: for an alias or a path below one it is the `*name` token.
10. **Performance guard.** A unit test in `positions.rs` asserts
    `simplified::yaml_scalar::alias_search_work()` (thread-local, available
    under `cfg(test)`) is unchanged across `build_position_map` of a document
    with 200 alias uses, proving no per-alias anchor search or re-expansion.
    No benchmark or spike is added.
11. **No spike.** Every question was answered by reading code or the
    scratch probe; there is no runtime measurement the spec asks for.
12. **Defects found in the shared locator by the new tests are fixed in
    `simplified/source.rs` in this change**, minimally, with a regression test;
    a wrong position is never hidden by suppression. (Probe evidence suggests
    none exist.)

### Wave 1 (sequential; one agent)

- [ ] **Orders iterator**
    - Add `pub(crate) fn pointers(&self) -> impl Iterator<Item = &str>` to
      `MappingOrders` (`mapping_orders.rs`), with a one-line doc.
- [ ] **Projection rewrite**
    - In `positions.rs`, change `matches_orders` to take `&mut SchemaValueNode`
      and a `&mut HashSet<String>` of boundaries; for a `Scalar` whose
      `alias_token(text, span.start)` is `Some(range)`: set `node.span = range`,
      insert the current pointer, return `true`; other scalars return `true`.
      Mapping and Sequence arms recurse through `with_child` as today (the
      mapping arm still compares `keys.len()` and each key in order).
    - In `PositionMap::project` make `root` `mut`, build the set, and replace
      `mappings != orders.len()` with `mappings + covered(orders, &boundaries)
      != orders.len()` where `covered` is a private fn implementing Rule 4.
    - Apply Rule 9 comment fixes in the same edit.
    - Prerequisite: Rule 1-4 above. Risk: `with_child` currently takes
      `FnOnce(&mut String) -> bool`; the new `&mut` borrows of node/set must
      move into the closure per entry (iterate `entries.iter_mut()`).

### Wave 2 (parallel; two agents, different files)

- [ ] **Unit tests in positions.rs**
    - Rewrite the two tests named in Rule 8, add the Rule 10 work test, and
      add: `a_projected_alias_leaf_spans_only_its_token` (block `one: *b # note`
      and flow `{k: *b }`: `byte_range` text is `*b`), and
      `a_trailing_comment_and_a_quoted_or_block_star_are_not_aliases`
      (`# *b`, `"*b"`, `|\n  *b` neither add boundaries nor change located
      ranges; the document stays projected).
- [ ] **Public-API tests**
    - New file `simplified-schema/lib/tests/l1/alias_positions.rs`, registered
      with `mod alias_positions;` in `tests/l1/main.rs`. It uses only the public
      API: a schema parsed with `SimplifiedSchema`, `validate_soft`/
      `validate_strict` on the `serde_yaml_ng`→JSON value to get
      `ValidationProblem`s, `build_position_map(yaml)`, `locate_all`,
      `byte_range`, and `PositionMap::project` with a hand-built
      `MappingOrders` (via `MappingOrders::collect` of a *different*
      `serde_yaml_ng::Value`) for refusal cases.
    - Control row (unedited, no alias): positions as today.
    - Spec example 1 (`servers` with `*b` item): sole problem `/servers/1/port`
      at (4, 12), range text `x`.
    - Spec example 2 (three uses): `/base/port` (1, 17) text `x`;
      `/one/port` (2, 6) and `/two/port` (3, 6), each range text `*b`
      (assert absolute byte start of each token too).
    - Alias to scalar (`a: &x 1` / `b: *x`), to sequence (`a: &x [1]` /
      `b: *x`, failure at `/b/0`), to mapping; sequence root
      (`- &a x\n- *a`, `- base: &b\n    x: 1\n- *b`) projected; alias in a flow
      sequence and in a flow mapping value; repeated uses.
    - Missing child under an alias, and an unknown key under an alias
      (`Key` target): both report the alias token, with distinct pointers
      among several failing children sharing one range.
    - Escaped keys: `"a/b": &x {p: 1}` with `c: *x`, and `"a~b"`; pointers
      `/c/p` resolve; keys containing `/` and `~` compare as decoded segments.
    - Literal merge: `base: &b {port: x}\nserver: {<<: *b, port: 2}`,
      quoted `"<<": *b`, and `<<: [*a, *b]`: `/server/port` locates at the
      explicit `2`; `/server/<</port` locates at its alias token; key order
      from `MappingOrders` unchanged; no inherited `/server/port` from `base`.
    - Refusal: `"&k a: 1\nb: 2\n"` and `"a: *b\n"` give un-located maps
      (`byte_range` is `None`, `locate_all` falls back to column 1). Boundary
      precision: YAML `a: &x {p: 1}\nab: {q: 1}\n` projected against orders
      collected from a document where `/ab` has different keys is refused
      (`project` is `None`), while the same YAML against its own orders is
      accepted. A key-order mismatch under a non-alias mapping beside an alias
      refuses.
    - Unicode: `é` before a value gives byte column (not character column);
      CRLF text and `project` with a nonzero `base_line` keep correct lines.
    - Prerequisite: Wave 1.

### Validation checkpoint 1

- [ ] In `simplified-schema/`: `just test` and `just lint` pass.
- [ ] Grep `simplified-schema/` for `"holds no alias"`, `contains an alias`,
      `no scalar is an alias`: zero stale comment hits in `src/`.

## Phase 2: Darkmatter integration

Goal: `load_data`, frontmatter, whole-file YAML validation, CLI, and DMLS all
show the improved locations. Prerequisite: Phase 1 complete.

### Necessary Rules

13. **`Format::Yaml` carries a plain `PositionMap`, not `Option<PositionMap>`.**
    With the suppression gone the `Option` encoded only the load-failure case,
    which becomes `PositionMap::new()` (its `byte_range` is `None`, identical
    to the old `None`). `offset()` then calls
    `positions.byte_range(...)` directly. `loaded.positions` may itself be the
    top-level fallback; `byte_range` already answers `None` for it, so the
    comment "never from the top-level-keys fallback" stays true.
14. **Delete `has_multi_line_flow_collection` and the `SchemaValueKind` import**
    (`locate.rs:19`, its only use is that helper). Rewrite the module doc and
    the `Format::Yaml` variant doc to say positions come from the shared map
    whenever the authored tree can be proven, including aliases (alias token)
    and multi-line flow collections; an unproven tree is pointer-only.
15. **No other source changes in Darkmatter.** Evidence: the
    `byte_range`/`positions` flow in `load.rs` (`LoadedYaml::byte_range`),
    `schemas/mod.rs:2362` (`positions_for`), `envelope.rs:129`, and DMLS
    `diagnostics/frontmatter.rs:514` all consume `PositionMap` by its public
    API and pick up the better map unchanged. `order_step` already orders by
    `authored_index` of the parsed value, so ordering cannot change.
16. **Coordinate contract is asserted, not changed.** `offset()` yields a byte
    offset; `place()` converts via `biscuit_file::source_position`, so
    `data_location.column` is a character column. Tests assert both
    conventions (character column in `load_data`, byte column in the shared
    map).
17. **Frontmatter coverage goes in the shared `PositionCase` table**
    (`darkmatter/lib/tests/position_fixtures/mod.rs`, included by the lib
    suite `tests/l1/diagnostic_positions.rs`, the CLI suite
    `cli/tests/l1/diagnostic_positions.rs`, and DMLS
    `dmls/src/diagnostics/frontmatter/position_parity_tests.rs`), which each
    run LF and CRLF. Add two cases, listed in `CASES`:
    - `alias_use`: document
      `---\nbase: &b {port: x}\none: *b\ntwo: *b\n$schema:\n  base: "{ port: number }"\n  one: "{ port: number }"\n  two: "{ port: number }"\n---\nBody\n`
      expecting `/base/port` kind `type` at (2, 17), `/one/port` (3, 6),
      `/two/port` (4, 6).
    - `alias_beside_unrelated_failure`: document
      `---\nbase: &b {port: 1}\nservers:\n  - *b\n  - {port: x}\n$schema:\n  base: "{ port: number }"\n  servers: "{ port: number }[]"\n---\nBody\n`
      expecting only `/servers/1/port` kind `type` at (5, 12).
    The existing `expected_starts_address_the_authored_tokens` test needs no
    change (it only checks `offending_property`).
18. **Existing Darkmatter tests that encode old behavior are rewritten:**
    `locate.rs` tests `yaml_orders_by_authored_index_even_without_positions`
    and `only_a_flow_collection_across_lines_makes_yaml_pointer_only`;
    `load_data_diagnostics.rs` `pointer_only!` entries
    `a_yaml_alias_reports_pointers_only` and
    `a_yaml_multi_line_flow_sequence_reports_pointers_only` (the `pointer_only!`
    macro and its section are removed if no entry remains); the doc comment of
    `load_data_diagnostics.rs` or `yaml_document_loading.rs` that says "even
    when the file has an alias" is kept (it is about load errors and stays
    true). Malformed-input tests (`load_data_diagnostics.rs` ~723-745,
    `load_data_execution.rs:399`) are NOT modified.
    Any further test the full suite shows asserting old alias/flow behavior is
    updated to the new behavior in this phase.
19. **Un-located control for Darkmatter:** the data fixture `"&k a: 1\nb: 2\n"`
    with contract `{ a: number, b: string }` (`/a` conforms, `/b` mismatches)
    yields one record `/b` with `line` and `column` both `None`
    (pointer-only), pointer still present. This is the "remaining
    unproven-position fallback" test; `&k a` is un-located in the shared map
    today (Rule 7) and the change does not touch anchored keys.

### Wave 3 (one agent)

- [ ] **Remove flow suppression**
    - Apply Rules 13, 14 in `locate.rs`; rewrite its two tests per Rule 18:
      the first becomes `yaml_orders_by_authored_index_and_places_an_alias`
      (text `base: &b {x: 1}\nzeta: *b\nalpha: 2\n`: `zeta` position
      `Some((2, 7))`, `alpha` `Some((3, 8))`, `zeta.order < alpha.order`);
      the second becomes
      `a_multi_line_flow_collection_keeps_yaml_positions`
      (`"items: [\n  x\n]\n"`: `place([items, 0])` position `Some((2, 3))`),
      plus the Rule 19 un-located case asserting `position == None` while
      `order` is still authored.

### Wave 4 (parallel; five agents, five files)

- [ ] **load_data library tests**
    - File `darkmatter/lib/tests/l1/load_data_diagnostics.rs`; use existing
      helpers `load`, `places`, `place`, `only`, `assert_all_mismatches`.
    - Spec example 1 and spec example 2 with the contracts from the spec
      (`"{ base: { port: number }, servers: { port: number }[] }"`;
      `"{ base: { port: number }, one: { port: number }, two: { port: number } }"`):
      exact pointers and (line, column) per the Definition of Done; the
      three-use case yields three records in authored order (`/base/port`,
      `/one/port`, `/two/port`); a missing child (contract adds
      `extra: number(required)` under `one`'s object) keeps its pointer
      (`/one/extra`) at the alias position and sorts after `/one/port`.
    - Multiple failing children of one alias share the alias position with
      distinct pointers.
    - Alias to scalar, sequence, mapping; neighbor values keep positions.
    - Literal `<<` (inline contracts cannot name a `<<` property, so use a
      pattern-key contract): data
      `base: &b {port: x}\nserver: {<<: *b, port: 2}` with contract
      `{ base: { port: number }, server: { <string>: { port: number } } }`.
      Records: `/base/port` (1, 17); `/server/<</port` at the alias,
      (2, 14); `/server/port` at the explicit `2`, (2, 24) (a number where an
      object is required). The quoted spelling `server: {"<<": *b, port: 2}`
      gives (2, 16) and (2, 26). A sequence of aliases used as a literal value,
      `base: &b {port: x}\nlist: [*b, *b]` with `{ base: { port: number }, list: { port: number }[] }`,
      gives `/list/0/port` (2, 8) and `/list/1/port` (2, 12).
    - Multi-line flow: `items: [\n  {port: x},\n  *b, # ]\n  "}"\n]` shapes —
      sequence of objects, nested flow mappings, comments and quoted `]`/`}`,
      an alias item, positions inside and after the collection (a key after
      the closing `]` keeps its position).
    - Coordinates: Unicode fixture `base: &b {port: x}\none: {note: é, link: *b}`
      with contract `{ base: { port: number }, one: { note: string, link: { port: number } } }`:
      `/one/link/port` is (2, 22) (character column; the byte column is 23);
      LF, CRLF, leading BOM, and BOM+CRLF variants of the spec-example-2 data
      all report identical (line, column) triples.
    - Ordering and counts: assert the record count and pointer order equal the
      same data with the aliases pre-expanded by hand into plain mappings (the
      same `pointers` list).
    - Replace the `pointer_only!` entries per Rule 18; add the Rule 19
      fallback test.
    - Malformed-input behavior stays in its own existing tests.
- [ ] **load_data CLI wiring tests**
    - File `darkmatter/cli/tests/l1/load_data.rs`, using `CliProcessFixture`,
      `json_run`, `only_record`, `read_entry`, `validate_strictly`.
    - Control: `base: &b {port: 1}\none: *b\n` against
      `{ base: { port: number }, one: { port: number } }` composes
      successfully (exit 0).
    - Alias: `base: &b {port: x}\none: *b\n` (exit 1): two
      `dm.data.contract_mismatch` records whose `data_location` objects are
      `{file, pointer: "/base/port", line: 1, column: 17}` and
      `{file, pointer: "/one/port", line: 2, column: 6}`; the envelope
      validates against `schemas/output/compose.yaml`; the later `::shell`
      sentinel does not run: assert `assert_failed(&envelope)` (outcome
      `failure`, `document` null) and the read set names only the data file.
    - `source` unchanged: for the alias run and a plain-data run with the same
      single failing value, `record["source"]` is equal.
    - Multi-line flow: `items: [\n  1,\n  x\n]\n` against
      `{ items: number[] }` reports `/items/1` at line 3, column 3.
- [ ] **Frontmatter shared cases**
    - Add the two `PositionCase`s of Rule 17 to
      `darkmatter/lib/tests/position_fixtures/mod.rs` and list them in `CASES`.
      This covers lib, CLI, and DMLS frontmatter parity with LF and CRLF.
- [ ] **Whole-file YAML validation test**
    - File `darkmatter/lib/tests/l1/yaml_document_loading.rs`: a document with
      inline `$schema` (`name: string(required)`,
      `base: "{ port: number }"`, `one: "{ port: number }"`), data
      `base: &b {port: x}` and `one: *b`. Loop the existing
      BOM / CRLF / BOM+CRLF / Unicode variants. Assert two problems with
      `problem.line` equal to the authored lines, and `&text[byte_range]` equal
      to `x` and `*b`. Add a control row (`port: 8080`-style valid) that
      validates.
- [ ] **DMLS alias range regression**
    - File `darkmatter/dmls/tests/l1/yaml_document_editor.rs`, using `Probe`,
      `ROSTER_DEFINITION`, `schema_diagnostics`, `start`, `position_of`,
      `only`/`codes`: text
      `kind: roster\nroster_version: 1\nplatforms:\n    - &p {name: [1]}\n    - *p\n`
      opened as a document. Expect two `dm.schema.kind_check`-graded
      diagnostics (no other schema diagnostics): one starting at
      `position_of(text, "[1]")` and one at `position_of(text, "*p")`, the
      latter's range ending exactly after `*p` (character length 2).
      A second probe adds `owner: é` before `platforms:` and asserts the
      start columns are UTF-16 code units (`é` is one unit), which is the
      `Probe`'s negotiated encoding. `name: [1]` is an array where the
      contract requires a string, so both diagnostics are type mismatches.

### Validation checkpoint 2

- [ ] In `darkmatter/`: `just test` and `just lint` pass. All new tests are
      L1, so `just test-l2` is not run.
- [ ] Grep `darkmatter/` and `.claude/skills/` for `has_multi_line_flow_collection`
      and `pointers only` / `pointer-only`: remaining hits are only in
      historical specs under `features/` and `fixes/`.

## Phase 3: Documentation and verification

Prerequisite: Phases 1 and 2.

### Necessary Rules

20. **Docs state behavior in their own words and never link to a
    feature/fix.** Evidence: repo CLAUDE.md "a `docs/` page never links to,
    or names, a feature or fix". The historical spec
    `2026-11-04-loading-data` is not edited.
21. **Docs to change** (audience: a developer new to the repo; explain what an
    alias is, give one example per rule):
    - `darkmatter/docs/cli/data-contracts.md`, section "YAML positions that
      cannot be proven" (~line 779-805): remove the "Pointer only" bullet and
      the "These are located even in a file with an alias" lead-in. New text:
      an alias-use example (the three-use YAML with the 1:17 / 2:6 / 3:6
      table), multi-line flow collections are located, `<<` is a literal key
      (no inheritance; `/server/<</port` points at the alias), and the
      remaining fallback (a source layout the locator cannot prove, e.g. an
      anchored key `&k a: 1`) is pointer-only. Keep the duplicate-key,
      second-document, non-string-key, and tag bullets.
    - `darkmatter/docs/topics/loading-data.md` (~lines 527-537, "YAML
      positions that cannot be proven"): same replacement, shorter.
    - `simplified-schema/docs/topics/diagnostics.md` (after the
      `byte_range` paragraph near line 261): add that an alias, or a path
      below one, is reported at the `*name` token and that a document the
      projection cannot prove falls back to top-level keys.
    - `.claude/skills/darkmatter/schema.md` (~lines 318-322 "The alignment
      witness"): drop "and it holds no alias", say aliases are leaves located
      at their token and indexed mappings below them are accounted for;
      fallback remains for unproven layouts (`<<` is no longer a fallback
      reason; unquoted `{{ }}` and anchored keys are).
    - Do not edit the `simplified-schema` skill (it does not describe alias
      refusal; verified by grep).
22. **Implementation log.** Create
    `darkmatter/features/2026-11-05-yaml-locator-hardening/implementation-log.md`
    (frontmatter in the style of the sibling
    `2026-11-05-render-surface/implementation-log.md`) recording: phases done,
    the intentional replacement of the whole-file pointer-only exception from
    `2026-11-04-loading-data`, the `Format::Yaml` `Option` removal (Rule 13),
    and any locator defect fixed under Rule 12. The spec is left unchanged.
23. **Verification scope.** `just test` and `just lint` in `simplified-schema/`
    and `darkmatter/` (nextest through the recipes). Because
    `positions.rs` is a dependency of the Darkmatter CLI and DMLS, also run
    `just test` in `darkmatter/` for all sub-areas it covers
    (lib, cli, dmls); no per-OS code is added and no CI plan change is
    needed. No L2/L3 tests are run or added.

### Wave 5 (parallel; four agents, four files)

- [ ] **Data contracts doc**: `darkmatter/docs/cli/data-contracts.md` per Rule 21.
- [ ] **Loading-data doc**: `darkmatter/docs/topics/loading-data.md` per Rule 21.
- [ ] **Diagnostics doc and skill**:
      `simplified-schema/docs/topics/diagnostics.md` and
      `.claude/skills/darkmatter/schema.md` per Rule 21.
- [ ] **Implementation log**: write it per Rule 22 (leave the verification
      section for Wave 6 to complete).

### Wave 6 (sequential)

- [ ] **Full verification**
    - Run `just test` and `just lint` in `simplified-schema/` then
      `darkmatter/`; record results in the implementation log.
    - Re-read every comment and doc touched for drift (CLAUDE.md "Authoring
      discipline"): `positions.rs`, `mapping_orders.rs`, `locate.rs` module
      docs; assume the code is right and fix the comment.
    - Final grep for stale claims across `*.md` and `*.rs` outside
      `features/` and `fixes/`: `holds no alias`, `file with an alias`,
      `contains an alias`, `Pointer only`, `pointer-only`,
      `multi-line flow`. Each remaining hit must be a true statement.

### Validation checkpoint 3 (final)

- [ ] All Definition of Done items hold and are covered by named tests.
- [ ] Implementation log lists tests added per acceptance criterion and the
      terminal state "implementation complete, ready for review" (never move
      the feature to `_completed`, never commit).
