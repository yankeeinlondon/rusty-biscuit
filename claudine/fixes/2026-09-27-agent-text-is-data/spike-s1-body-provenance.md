---
created: 2026-09-28
spike: S1
---

# Spike S1: body provenance

Paths are relative to `darkmatter/lib/src/markdown/compose/` unless they say
otherwise. The `md` evidence came from the installed binary, run against
fixtures in `/tmp/s1` with a `.darkmatter-shell-whitelist` that allows only
`echo` and `printf`.

## Decision: (a) tracked data ranges

Use **(a)**: track data-origin byte ranges through every body rewrite, next to
the existing `BodyOrigin` in `body_origin.rs`.

Reasons:

1. **Every operation that scans can remap.** Each body stage up to the
   transclusion directive parse already rewrites through span-based
   replacements:
   - `TextEdit` in `replacement.rs:97`, `page_blocks/engine.rs:32`, and
     `directive_targets.rs:354`
   - `(span, String)` vectors in ShellExpansion, ShellBlocks, and LinkResolve

   Emitting `TextEdit`s from those vectors is mechanical.
2. **The operations that cannot remap run after the last scan.** Cleanup,
   fixed-width reflow, Normalization, and the transclusion post-cache
   transforms rewrite the whole text:
   - `relevel_with_overflow`
   - `apply_wrappers`, which prefixes each line
   - `remove_sections`

   None of them looks for instructions. The parent parses its `::file`,
   `::code`, `::toc-linking`, and `::file-links` directives *before* the
   splice (`pipeline/phases.rs:196-229`), and a child's composed text is never
   scanned by the parent. So provenance can end after the transclusion parse,
   before the first operation that cannot remap.
3. **Option (b) breaks operations that must see the real text.** These are:
   - LinkResolve and LinkNormalization: `[x]({{ path }})` is a common pattern,
     and its target must be resolved and then normalized.
   - Reference-graph fragment slugs and `md toc` heading slugs: headings
     inside data disappear behind a placeholder.
   - Cleanup: table alignment and `fixed_width` reflow measure width, and a
     placeholder is one character wide.
   - Normalization and transclusion relevel: headings inside data must still
     be releveled.
   - Multi-line indentation of replacement text.
   - The run-local compose cache (`ComposeResult.content` is a `String`):
     cached child text would need its arena cached and renumbered in every
     parent.
   - Private-use collisions: shell output and authored text may already
     contain private-use code points, such as Nerd Font glyphs, so (b) needs
     its own escape scheme.
4. **(b)'s one advantage has a cheap equivalent under (a).** Under (b), data
   cannot change how *authored* structure is parsed. Today it can: a data
   `` ``` `` hides a following authored `::shell` line (evidence E6). Under (a),
   use a **masked view**: copy the text, then replace every non-whitespace
   byte inside a data range with `x`. Offsets and line breaks are unchanged,
   and the copy is still valid UTF-8 because whole characters are replaced.
   Code-region detection runs over the masked view. The single choke point is
   `find_code_regions` (`parse_utils.rs:397`), plus the lexer's copy at
   `expression/lexer.rs:298`.

**Fail closed.** `BodyOrigin` is advisory: when it no longer describes the
text, it is silently dropped (`advance`, `body_origin.rs`). Dropping data
ranges would silently turn data back into instructions. `DataRanges` must
therefore check `describes()` at every scanner. A mismatch is an internal
`MarkdownError`, never a fallback to "all authored".

## Evidence: data re-enters the scanners today (body)

| # | Fixture (frontmatter → body) | `md compose` output | What it shows |
|---|---|---|---|
| E1 | `x: "::shell echo INJECTED"` → `{{ x }}` | `INJECTED` (and `--shell` lists `echo INJECTED`, line 6) | ShellExpansion and preflight scan data |
| E2 | `x: "::file ./child.md"` → `{{ x }}` | `CHILD CONTENT child` | Transclusion parse scans data |
| E3 | `x: "::code ./child.md"` → `{{ x }}` | a fenced copy of `child.md` | Same |
| E4 | `x` = a `::shell-block … ::end-block` → `{{ x }}` | `BLOCK` | ShellBlocks scans data |
| E5 | `x: "hi\necho SECOND"` inside an authored `::shell-block` `echo {{ x }}` | `hi SECOND`; `--shell` lists `echo hi` **and** `echo SECOND` | A newline in data creates a new command |
| E5b | `x: "\n::end-block\n…"` inside a shell block | `Unmatched ::end-block at line 5` | Data closes authored structure |
| E5c | `y: echo` → `::shell {{ y }} EXEC-FROM-DATA` | `EXEC-FROM-DATA` | An interpolated executable runs in the body |
| E6 | `` x: "```" `` (a bare fence) → `{{ x }}` then authored `::shell echo AUTHORED` | a text fence swallowing the directive | Data changes how authored text is parsed |
| E7 | `{{ frontmatter("./other.md","m") }}`, where `m: "see {{ b }} here"` | `see SECRET here` | A file read is rescanned in the reader's scope |
| E8 | Same, with `k: "a {{{ b }}} c"` | `a {{ b }} c` | `convert_literals` rewrites data after insertion (corruption) |
| E9 | `x: "[l](./nope.md)"` → `see {{ x }}` | `Invalid Hyperlink(s)`, exit 2 | Reference validation fails the run on a data link |
| E10 | `{{ flag ? "see {{ c }}" : "none" }}` | `see x` | A body ternary relies on the rescan (I1 migration) |

## Operations that must honor data ranges

"Scans" means the operation looks for instructions, so it must skip data.
"Remap" means it must report `TextEdit`s.

| Operation | Site | Rewrites? | Remap | Scans? | What it must do |
|---|---|---|---|---|---|
| TextReplacement | `inline/replacement.rs:21`, `replacement.rs:97` | yes | has edits | no (runs before any body data) | Inserted text takes the origin of its `replace.*` leaf (N2 data paths). An interpolated leaf is Data. Covers `one_off_replace` from a parent directive. |
| PageBlocks | `inline/page_blocks.rs:25,54` | removes regions | has edits | yes: `::block` markers, `when=` | Ignore markers whose line prefix or keyword lies in data (possible after a Data replacement). Carry edits. |
| Directive targets | `directive_targets.rs:354`; called at `inline/interpolation.rs:63` | yes | has edits | yes | Only authored `::file {{ … }}` targets. The evaluated target is Data. |
| Interpolation | `inline/interpolation.rs:74`; `interpolation/rewrite.rs:148-293` | yes | **add** | yes | Single pass. Skip any location that intersects data (targets output, Data replacements). Emit one Data edit per replaced expression and per converted literal. |
| `convert_literals` | `rewrite.rs:89`, called at `:284` | yes | **add** | yes (full rescan of the output) | Fold into the same single scan (`ExpressionFinder::scan` already returns `literals`) so it never touches inserted data (E8). Output is Data. |
| ShellExpansion | `inline/shell_expansion.rs:22,60`; `shell_expansion/parser.rs:33-53` | yes | **add** (from `replacements`) | yes | A directive counts only when its line prefix and `::shell` keyword are authored. The **executable token must be authored** (E5c). Output is Data. |
| ShellBlocks | `shell_blocks/mod.rs:46,90`, `:214`; `block_pairs.rs:68`; `body::split_logical_commands` | yes | **add** (return `replacements`) | yes | Open and `::end-block` markers must be authored (E4, E5b). A command boundary (a newline or a `\` continuation) inside data does not split commands (E5), and each executable token must be authored. Output is Data. |
| LinkResolve | `link_resolve.rs:91-131` | yes (link targets) | **add** | no | Still resolves links inside data (compatibility). An edit inside data stays Data (`EditOrigin::Inherit`). |
| Code-region detection | `parse_utils.rs:397`, `expression/lexer.rs:298`, `directives_api.rs:151` | no | n/a | structural | Run over the masked view (E6). |
| Transclusion parse | `pipeline/phases.rs:202-229`; `transclusion/parser.rs:45`; `toc_linking/parser.rs:13`; `file_links/parser.rs:18` | no | n/a | yes | Skip directives whose marker is in data (E2, E3). **Last consumer**: drop `DataRanges` after this parse. |
| Transclusion splice | `phases.rs:445-503`; `transclusion/engine.rs:1104` (remote), `:1487` (local) | yes | not needed | no | The child's text is final and nothing in the parent rescans it. The child's **own** pipeline tracks its data. Parent Data leaves cross into the child through `build_child_external_state` (`engine.rs:1637`) and must bring their data paths with them. |
| Compose cache key | `cache/hashing.rs:96` (`PhaseStateIdentity`) | n/a | n/a | n/a | Hash the frontmatter data-path set. Identical values with a different origin now compose differently (Data replace leaf vs authored). |
| Cleanup, reflow | `pipeline/phases.rs:87-139` | whole text | **cannot** | no | No provenance needed; it has already ended. |
| Normalization | `phases.rs:141` | headings | could | no | Same. Headings inside data are still releveled (desired). |
| LinkNormalization (Finalization) | `link_normalization.rs` | yes | could | no | Same. Data links are normalized back to portable form. |
| TOC (`md toc`, `toc()`) | `Markdown::toc` | no | n/a | no | Headings inside data are content, so their slugs stay unchanged. Nothing to do under (a). |
| Reference graph and validation | `../reference/graph.rs:884-896` (`prepare_content`), `validate.rs:771` | InlinePre only | n/a | yes | `prepare_content` must return data ranges. Skip `::file` edges declared in data. Links inside data: **decision needed** (E9). Recommend a warning, not an error. |
| Preflight collection | `preflight/collect.rs:373-395` | InlinePre only | n/a | yes | Use the same data-aware parsers as runtime, so the approval set matches what executes (E1, E5). |
| Lifecycle post-guard (Claudine) | `claudine/lib/src/composition/lifecycle/executor.rs:2050` | n/a | n/a | yes | Gets `data` from `InterpolationRewrite`, so the guard checks only authored spans (R1.4). Owned by Phase 4, but the API comes from here. |

## Prototype diff summary

`body_origin.rs`:

- `TextEdit` gains `origin: EditOrigin`, where `EditOrigin` is `Authored`,
  `Data`, or `Inherit`. `Inherit` becomes Data when the replaced range
  intersects data, so the conservative result is always inert.
- Add `pub(crate) struct DataRanges { ranges: Vec<Range<usize>>, text_len, text_hash }`
  with these methods:
  - `after_edits(&self, &[TextEdit], output) -> Self` (the same forward pass
    as `BodyOrigin`)
  - `intersects(&Range) -> bool`
  - `ensure_describes(&str) -> MarkdownResult<()>`, which fails closed
  - `masked(&str) -> Cow<str>`
- Add `pub(crate) struct BodyProvenance { origin: Option<BodyOrigin>, data: DataRanges }`.
  It replaces the `&mut Option<BodyOrigin>` parameter of
  `run_inline_pre_operation` (`phases.rs:29`).
  `advance(&mut BodyProvenance, before, &edits, after)` updates both fields.
  `BodyOrigin` keeps its best-effort drop.
- Seed the data ranges from TextReplacement using the N2 frontmatter
  data-path set.

`interpolation/rewrite.rs`:

- Delete `MAX_INTERPOLATION_DEPTH`, the `for depth` loop, and `reported` /
  `shift_reported`. The loop exists only to find expressions in replacement
  output.
- `ExpressionOrigin::Generated` (`context/report.rs:622`) becomes dead.
  `LocatedInterpolationError.span` is always `Some`.
- `interpolate_text_located(input, input_data: &DataRanges, …)` scans once
  with `ExpressionFinder::scan`. It applies expressions and literals together
  from end to start, and skips any location that intersects `input_data`.
- `InterpolationRewrite` gains `edits: Vec<TextEdit>` (all Data).
  `interpolate_text` and `interpolate_value` keep their signatures and pass
  empty data. Frontmatter callers gain the per-leaf data check from N2 instead.

Scanner helpers:

- Add one `authored_directive_at(line_start, prefix_len, keyword_len, &DataRanges) -> bool`.
- Add `find_code_regions_masked(content, &DataRanges)`.
- Thread `&DataRanges` into these parsers:
  - `shell_expansion::parse_directives`
  - `block_pairs::scan_block_pairs`
  - `shell_blocks::body::split_logical_commands` (command boundaries, the
    executable token)
  - `page_blocks::parser`
  - `transclusion::parse_directives`
  - `toc_linking::parse_directives`
  - `file_links::parse_file_links_directives`

  The public `parse_directives` and `directives_api` keep their signatures
  through a wrapper that passes empty data.

Stage runners:

- `inline/shell_expansion.rs`, `shell_blocks::run_shell_blocks_stage`, and
  `link_resolve` return `Vec<TextEdit>`. The transclusion phase takes
  `BodyProvenance` by value and drops it after the parse.
- Add a crate-private `compose_inline_pre_prepared(options) -> (Markdown, DataRanges)`.
  `reference/graph.rs::prepare_content` and `preflight/collect.rs:373` use it.

**Tests to add in Phase 2:**

- E1–E8 and E5b/E5c as L1 tests, asserting data is inert and the authored
  equivalents still run.
- A `DataRanges::after_edits` property test against a naive per-byte origin
  model.
- A masked-view test: data `` ``` `` followed by an authored `::shell`.
- The describes-mismatch path returns an internal error.

## Frontmatter key-to-key references

**They are resolved in dependency order, not by the rescan.**
`interpolate_frontmatter_impl` (`frontmatter_interpolation.rs:696`) works like
this:

1. It extracts each templated key's references once
   (`extract_frontmatter_key_refs`, `:1295`).
2. It keeps `dep_count` and `dependents`, then sweeps until nothing changes
   (Kahn-style, `:781-935`).
3. As each key resolves, its value goes into `state.data`, so dependents look
   up the *resolved* value.

Unit test `wide_and_deep_graph_resolves_in_dependency_order` pins this.

Empirical proof. In `typed.md`, `a` is declared first, and its evaluation
succeeds only if `b` was already a number:

```text
$ md compose typed.md --fm      # a: "{{ b * 2 }}", b: "{{ c + 1 }}", c: 5
a: 12
b: 6
c: 5
$ md compose chain.md --fm      # a: "{{ b }}", b: "{{ c }}", c: x   -> a: x, b: x, c: x
$ md compose chain-rev.md --fm  # same keys declared c, b, a        -> same output
$ md compose chain4.md          # a: "pre-{{ b }}" … d: x           -> A=pre-mid-end-x
```

**Cycles are silent today.** Keys in a cycle never reach `dep_count == 0`, so
the fallback pass (`:943`) evaluates them with the partner missing from
`state.data`:

```text
$ md compose cycle.md --fm     # a: "{{ b }}", b: "{{ a }}"
a: null
b: null                        # no warning, exit 0
$ md compose selfcycle.md --fm # a: "x{{ a }}"  (self-refs are not dependencies)
a: x
```

**The rescan does appear, but only where resolved text contains `{{`:**

```text
$ md compose fmmixed.md --fm   # b: "{{ flag ? '{{ c }}' : '' }}", a: "pre {{ b }}"
b: '{{ c }}'                   # whole-value path: typed, not rescanned
a: pre x                       # mixed-text path: rescan of inserted '{{ c }}'
$ md compose pass2.md --fm     # adds s: "$(printf '\173\173 c \175\175')", d: "pre {{ s }}"
b: x                           # pass 2 rescans pass-1 data (was '{{ c }}')
s: x                           # pass 2 interpolates shell OUTPUT
d: pre x
$ md compose esc.md            # note: "fixed {{{ area }}}", body {{ note }}
Body: fixed claudine
```

`pass2-ctrl.md` is the same file without `s`. It keeps `b: '{{ c }}'`, which
shows that the rescan of `b` happens only when shell expansion replaced
something. Pass 2 (`pipeline/mod.rs:351-375`) re-partitions *every* key with
`contains_interpolation`, so N4 is confirmed.

**Does N2 need a topological order? No new one.** A dependent's authored
`{{ b }}` is scanned once, and `b` is *looked up*, not rescanned, so chains
keep working under "scanned at most once; result is data". N2 still needs two
changes to the existing order:

1. **A data-aware partition.** `templated_keys` (`:729`) and the seed split
   (`:735`) must classify by origin, not by `contains_interpolation`:
   - A Data leaf is seed even when it contains `{{`.
   - A key's references come only from its authored leaves.
   - `rewrite_value` (`:254`) skips Data leaf paths.
   - `convert_frontmatter_literals` (`:476`) skips Data leaves and marks the
     leaves it converts as Data.
2. **Pass 2 reuses the same ordered sweep**, restricted to the deferred
   authored leaves (N4). Deferred keys that reference each other still
   resolve in dependency order.

Cycles are unaffected by N2 because no rescan is involved. Turning them into
a warning is an optional, separate fix.

## Findings that affect rulings

- **N2:** the ruling stands, and no topological order needs to be added. Two
  amendments:
  - The body provenance must reach the reference graph and preflight through
    an InlinePre-only compose that returns `DataRanges`.
  - The frontmatter data-path set must be part of the transclusion cache key
    and cross into the child through `build_child_external_state`.

  "Flatten at Finalization" should read "drop after the transclusion
  directive parse". Cleanup cannot remap ranges, and it does not need to.
- **N4 is too narrow.** R1.2's "the command shape comes from authored source"
  also applies to the **body**:
  - An interpolated executable runs (E5c).
  - A data newline inside a `::shell-block` creates a new command, and
    preflight approves it (E5).
  - A data `::end-block` closes an authored block (E5b).

  Add a body clause: markers, command boundaries, and executable tokens must
  lie in authored bytes. Otherwise E5 is still reachable after Phase 2.
- **New decision (R5-adjacent):** a relative link inside data fails the run
  through reference validation (E9, exit 2). Recommend that data-origin link
  targets warn instead.
- **I1 migration:** the body-ternary-with-nested-template idiom (E10) and
  the mixed-text frontmatter form (`fmmixed`) change output under N2. Add both
  to the migration table.
